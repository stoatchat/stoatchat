use iso8601_timestamp::{Duration, Timestamp};

use nanoid::nanoid;
use revolt_config::config;
use revolt_result::{Result, ToRevoltError};
use serde_json::json;
use webauthn_rs::prelude::{
    AuthenticationResult, DiscoverableAuthentication, Passkey, PublicKeyCredential,
    RegisterPublicKeyCredential, Uuid,
};
use webauthn_rs_proto::ResidentKeyRequirement;

use crate::{
    Database, MFATicket, Session,
    events::client::EventV1,
    util::{
        email::{email_templates, normalise_email, send_email},
        password::hash_password,
        webauthn::{PasskeyChallenge, PasskeyChallengeState, encode_credential_id, webauthn},
    },
};
use revolt_models::v0;

auto_derived_partial!(
    /// Account model
    pub struct Account {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,

        /// User's email
        pub email: String,

        /// Normalised email
        ///
        /// (see https://github.com/insertish/authifier/#how-does-authifier-work)
        pub email_normalised: String,

        /// Argon2 hashed password
        pub password: String,

        /// Whether the account is disabled
        #[serde(default)]
        pub disabled: bool,

        /// Email verification status
        pub verification: EmailVerification,

        /// Password reset information
        pub password_reset: Option<PasswordReset>,

        /// Account deletion information
        pub deletion: Option<DeletionInfo>,

        /// Account lockout
        pub lockout: Option<Lockout>,

        /// Multi-factor authentication information
        pub mfa: MultiFactorAuthentication,
    },
    "PartialAccount"
);

auto_derived!(
    /// Email verification status
    #[serde(tag = "status")]
    pub enum EmailVerification {
        /// Account is verified
        Verified,
        /// Pending email verification
        Pending { token: String, expiry: Timestamp },
        /// Moving to a new email
        Moving {
            new_email: String,
            token: String,
            expiry: Timestamp,
        },
    }

    /// Password reset information
    pub struct PasswordReset {
        /// Token required to change password
        pub token: String,
        /// Time at which this token expires
        pub expiry: Timestamp,
    }

    /// Account deletion information
    #[serde(tag = "status")]
    pub enum DeletionInfo {
        /// The user must confirm deletion by email
        WaitingForVerification { token: String, expiry: Timestamp },
        /// The account is scheduled for deletion
        Scheduled { after: Timestamp },
        /// This account was deleted
        Deleted,
    }

    /// Lockout information
    pub struct Lockout {
        /// Attempt counter
        pub attempts: i32,
        /// Time at which this lockout expires
        pub expiry: Option<Timestamp>,
    }

    /// MFA configuration
    #[derive(Default)]
    pub struct MultiFactorAuthentication {
        /// Allow password-less email OTP login
        /// (1-Factor)
        // #[serde(skip_serializing_if = "is_false", default)]
        // pub enable_email_otp: bool,

        /// Allow trusted handover
        /// (1-Factor)
        // #[serde(skip_serializing_if = "is_false", default)]
        // pub enable_trusted_handover: bool,

        /// Allow email MFA
        /// (2-Factor)
        // #[serde(skip_serializing_if = "is_false", default)]
        // pub enable_email_mfa: bool,

        /// TOTP MFA token, enabled if present
        /// (2-Factor)
        #[serde(skip_serializing_if = "Totp::is_empty", default)]
        pub totp_token: Totp,

        /// Registered passkeys, enabled if present
        /// (1-Factor or 2-Factor)
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        pub passkeys: Vec<PasskeyCredential>,

        /// Recovery codes
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        pub recovery_codes: Vec<String>,
    }

    /// MFA method
    #[derive(Hash)]
    pub enum MFAMethod {
        Password,
        Recovery,
        Totp,
        Passkey,
    }

    #[derive(Default)]
    #[serde(tag = "status")]
    pub enum Totp {
        /// Disabled
        #[default]
        Disabled,
        /// Waiting for user activation
        Pending { secret: String },
        /// Required on account
        Enabled { secret: String },
    }
);

/// Passkey registered to an account
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PasskeyCredential {
    /// Unique Id
    pub id: String,
    /// Base64url encoded WebAuthn credential id
    pub credential_id: String,
    /// User provided name
    pub name: String,
    /// WebAuthn credential
    #[serde(with = "json_passkey")]
    pub credential: Passkey,
    /// Whether the client reported creating a discoverable credential (Resident Key).
    /// This is set in an insecure context and is not to be used for security purposes (ui hint only)
    #[serde(default)]
    pub discoverable: bool,
    /// When this passkey was registered
    pub created_at: Timestamp,
    /// When this passkey was last used to authenticate
    pub last_used: Option<Timestamp>,
}

/// (De)serialise a `Passkey` through its JSON representation
mod json_passkey {
    use serde::{
        Deserialize, Deserializer, Serialize, Serializer, de::Error as _, ser::Error as _,
    };
    use webauthn_rs::prelude::Passkey;

    pub fn serialize<S: Serializer>(passkey: &Passkey, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::to_value(passkey)
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Passkey, D::Error> {
        serde_json::from_value(serde_json::Value::deserialize(deserializer)?)
            .map_err(D::Error::custom)
    }
}

impl PartialEq for PasskeyCredential {
    fn eq(&self, other: &Self) -> bool {
        // `Passkey` doesn't implement `PartialEq`, compare its serialised form instead
        self.id == other.id
            && self.credential_id == other.credential_id
            && self.name == other.name
            && self.discoverable == other.discoverable
            && self.created_at == other.created_at
            && self.last_used == other.last_used
            && serde_json::to_value(&self.credential).ok()
                == serde_json::to_value(&other.credential).ok()
    }
}

impl Eq for PasskeyCredential {}

/// Passwordless login which has been identified but not yet verified
pub struct PasskeyLogin {
    state: DiscoverableAuthentication,
    response: PublicKeyCredential,
}

impl MultiFactorAuthentication {
    // Check whether MFA is in-use
    pub fn is_active(&self) -> bool {
        matches!(self.totp_token, Totp::Enabled { .. }) || !self.passkeys.is_empty()
    }

    // Check whether there are still usable recovery codes
    pub fn has_recovery(&self) -> bool {
        !self.recovery_codes.is_empty()
    }

    // Get available MFA methods
    pub fn get_methods(&self) -> Vec<MFAMethod> {
        if self.is_active() {
            let mut methods = vec![];

            if let Totp::Enabled { .. } = self.totp_token {
                methods.push(MFAMethod::Totp);
            }

            if !self.passkeys.is_empty() {
                methods.push(MFAMethod::Passkey);
            }

            if self.has_recovery() {
                methods.push(MFAMethod::Recovery);
            }

            methods
        } else {
            vec![MFAMethod::Password]
        }
    }

    /// Find a registered passkey by its WebAuthn credential id
    pub fn find_passkey_mut(&mut self, credential_id: &[u8]) -> Option<&mut PasskeyCredential> {
        let credential_id = encode_credential_id(credential_id);
        self.passkeys
            .iter_mut()
            .find(|passkey| passkey.credential_id == credential_id)
    }

    // Generate new recovery codes
    pub fn generate_recovery_codes(&mut self) {
        static ALPHABET: [char; 32] = [
            '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f', 'g',
            'h', 'j', 'k', 'm', 'n', 'p', 'q', 'r', 's', 't', 'v', 'w', 'x', 'y', 'z',
        ];

        let mut codes = vec![];
        for _ in 1..=10 {
            codes.push(format!(
                "{}-{}",
                nanoid!(5, &ALPHABET),
                nanoid!(5, &ALPHABET)
            ));
        }

        self.recovery_codes = codes;
    }

    // Generate new TOTP secret
    pub fn generate_new_totp_secret(&mut self) -> Result<String> {
        if let Totp::Enabled { .. } = self.totp_token {
            return Err(create_error!(OperationFailed));
        }

        let secret: [u8; 10] = rand::random();
        let secret = base32::encode(base32::Alphabet::RFC4648 { padding: false }, &secret);

        self.totp_token = Totp::Pending {
            secret: secret.clone(),
        };

        Ok(secret)
    }

    /// Enable TOTP using a given MFA response
    pub fn enable_totp(&mut self, response: v0::MFAResponse) -> Result<()> {
        if let v0::MFAResponse::Totp { totp_code } = response {
            let code = self.totp_token.generate_code()?;

            if code == totp_code {
                let mut totp = Totp::Disabled;
                std::mem::swap(&mut totp, &mut self.totp_token);

                if let Totp::Pending { secret } = totp {
                    self.totp_token = Totp::Enabled { secret };

                    Ok(())
                } else {
                    Err(create_error!(OperationFailed))
                }
            } else {
                Err(create_error!(InvalidToken))
            }
        } else {
            Err(create_error!(InvalidToken))
        }
    }
}

impl Totp {
    /// Whether TOTP information is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, Totp::Disabled)
    }

    /// Whether TOTP is disabled
    pub fn is_disabled(&self) -> bool {
        !matches!(self, Totp::Enabled { .. })
    }

    // Generate a TOTP code from secret
    pub fn generate_code(&self) -> Result<String> {
        if let Totp::Enabled { secret } | Totp::Pending { secret } = &self {
            let seconds: u64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            Ok(totp_lite::totp_custom::<totp_lite::Sha1>(
                totp_lite::DEFAULT_STEP,
                6,
                &base32::decode(base32::Alphabet::RFC4648 { padding: false }, secret)
                    .expect("valid base32 secret"),
                seconds,
            ))
        } else {
            Err(create_error!(OperationFailed))
        }
    }
}

impl Account {
    /// Save model
    pub async fn save(&self, db: &Database) -> Result<()> {
        db.save_account(self).await
    }

    /// Create a new account
    pub async fn new(
        db: &Database,
        email: String,
        plaintext_password: String,
        verify_email: bool,
    ) -> Result<Account> {
        // Get a normalised representation of the user's email
        let email_normalised = normalise_email(email.clone());

        // Try to find an existing account
        if let Some(mut account) = db
            .fetch_account_by_normalised_email(&email_normalised)
            .await?
        {
            // Resend account verification or send password reset
            if let EmailVerification::Pending { .. } = &account.verification {
                account.start_email_verification(db).await?;
            } else {
                account.start_password_reset(db, true).await?;
            }

            Ok(account)
        } else {
            // Hash the user's password
            let password = hash_password(plaintext_password)?;

            // Create a new account
            let mut account = Account {
                id: ulid::Ulid::new().to_string(),

                email,
                email_normalised,
                password,

                disabled: false,
                verification: EmailVerification::Verified,
                password_reset: None,
                deletion: None,
                lockout: None,

                mfa: Default::default(),
            };

            // Send email verification
            if verify_email {
                account.start_email_verification(db).await?;
            } else {
                account.save(db).await?;
            }

            // Create and push event
            EventV1::CreateAccount {
                account: account.clone(),
            }
            .global()
            .await;

            Ok(account)
        }
    }

    /// Create a new session
    pub async fn create_session(&self, db: &Database, name: String) -> Result<Session> {
        let config = config().await;

        let session = Session {
            id: ulid::Ulid::new().to_string(),
            token: nanoid!(64),

            user_id: self.id.clone(),
            name,

            last_seen: Timestamp::now_utc(),

            origin: Some(config.environment),
            subscription: None,
        };

        // Save to database
        db.save_session(&session).await?;

        // Create and push event
        EventV1::CreateSession {
            session: session.clone(),
        }
        .global()
        .await;

        Ok(session)
    }

    /// Send account verification email
    pub async fn start_email_verification(&mut self, db: &Database) -> Result<()> {
        let config = config().await;

        if !config.api.smtp.host.is_empty() {
            let templates = email_templates().await;

            let token = nanoid!(32);
            let url = format!("{}{}", templates.verify.url, token);

            send_email(
                &config.api.smtp,
                self.email.clone(),
                &templates.verify,
                json!({
                    "email": self.email.clone(),
                    "url": url
                }),
            )?;

            self.verification = EmailVerification::Pending {
                token,
                expiry: Timestamp::now_utc()
                    .checked_add(Duration::seconds(
                        config.api.smtp.expiry.expire_verification,
                    ))
                    .unwrap(),
            };
        } else {
            self.verification = EmailVerification::Verified;
        }

        self.save(db).await
    }

    /// Send account verification to new email
    pub async fn start_email_move(&mut self, db: &Database, new_email: String) -> Result<()> {
        // This method should and will never be called on an unverified account,
        // but just validate this just in case.
        if let EmailVerification::Pending { .. } = self.verification {
            return Err(create_error!(UnverifiedAccount));
        }

        let config = config().await;

        if !config.api.smtp.host.is_empty() {
            let templates = email_templates().await;

            let token = nanoid!(32);
            let url = format!("{}{}", templates.verify.url, token);

            send_email(
                &config.api.smtp,
                new_email.clone(),
                &templates.verify,
                json!({
                    "email": self.email.clone(),
                    "url": url
                }),
            )?;

            self.verification = EmailVerification::Moving {
                new_email,
                token,
                expiry: Timestamp::now_utc()
                    .checked_add(Duration::seconds(
                        config.api.smtp.expiry.expire_verification,
                    ))
                    .unwrap(),
            };
        } else {
            self.email_normalised = normalise_email(new_email.clone());
            self.email = new_email;
        }

        self.save(db).await
    }

    /// Send password reset email
    pub async fn start_password_reset(
        &mut self,
        db: &Database,
        existing_account: bool,
    ) -> Result<()> {
        let config = config().await;

        if !config.api.smtp.host.is_empty() {
            let templates = email_templates().await;

            let template = if existing_account {
                &templates.reset_existing
            } else {
                &templates.reset
            };

            let token = nanoid!(32);
            let url = format!("{}{}", template.url, token);

            send_email(
                &config.api.smtp,
                self.email.clone(),
                template,
                json!({
                    "email": self.email.clone(),
                    "url": url
                }),
            )?;

            self.password_reset = Some(PasswordReset {
                token,
                expiry: Timestamp::now_utc()
                    .checked_add(Duration::seconds(
                        config.api.smtp.expiry.expire_password_reset,
                    ))
                    .unwrap(),
            });
        } else {
            return Err(create_error!(OperationFailed));
        }

        self.save(db).await
    }

    /// Begin account deletion process by sending confirmation email
    ///
    /// If email verification is not on, the account will be marked for deletion instantly
    pub async fn start_account_deletion(&mut self, db: &Database) -> Result<()> {
        let config = config().await;

        if !config.api.smtp.host.is_empty() {
            let templates = email_templates().await;

            let token = nanoid!(32);
            let url = format!("{}{}", templates.deletion.url, token);

            send_email(
                &config.api.smtp,
                self.email.clone(),
                &templates.deletion,
                json!({
                    "email": self.email.clone(),
                    "url": url
                }),
            )?;

            self.deletion = Some(DeletionInfo::WaitingForVerification {
                token,
                expiry: Timestamp::now_utc()
                    .checked_add(Duration::seconds(
                        config.api.smtp.expiry.expire_password_reset,
                    ))
                    .unwrap(),
            });

            self.save(db).await
        } else {
            self.schedule_deletion(db).await
        }
    }

    /// Verify a user's password is correct
    pub fn verify_password(&self, plaintext_password: &str) -> Result<()> {
        argon2::verify_encoded(&self.password, plaintext_password.as_bytes())
            .map(|v| {
                if v {
                    Ok(())
                } else {
                    Err(create_error!(InvalidCredentials))
                }
            })
            // To prevent user enumeration, we should ignore
            // the error and pretend the password is wrong.
            .map_err(|_| create_error!(InvalidCredentials))?
    }

    /// Convert the user's ID to a UUID using the same bits as the ULID
    ///
    /// webauthn-rs only takes UUIDs at a high level. So here we are. Back at UUIDs...
    pub fn id_as_uuid(&self) -> Result<Uuid> {
        ulid::Ulid::from_string(&self.id)
            .map(|ulid| Uuid::from_u128(ulid.0))
            .to_internal_error()
    }

    /// Record a successful passkey authentication
    pub async fn record_passkey_use(
        &mut self,
        db: &Database,
        result: &AuthenticationResult,
    ) -> Result<()> {
        let passkey = self
            .mfa
            .find_passkey_mut(result.cred_id())
            .ok_or_else(|| create_error!(InvalidCredentials))?;

        if result.needs_update() {
            passkey.credential.update_credential(result);
        }
        passkey.last_used = Some(Timestamp::now_utc());

        self.save(db).await.to_internal_error()
    }

    /// Begin registering a new passkey
    ///
    /// Returns the challenge token and creation options
    pub async fn start_passkey_registration(
        &self,
        db: &Database,
    ) -> Result<(String, serde_json::Value)> {
        let config = config().await;

        if self.mfa.passkeys.len() >= config.api.security.webauthn.max_passkeys {
            return Err(create_error!(TooManyPasskeys {
                max: config.api.security.webauthn.max_passkeys
            }));
        }

        let user = db.fetch_user(&self.id).await?;
        let display_name = user
            .display_name
            .unwrap_or_else(|| format!("{}#{}", user.username, user.discriminator));

        let exclude_credentials = self
            .mfa
            .passkeys
            .iter()
            .map(|passkey| passkey.credential.cred_id().clone())
            .collect();

        let (mut options, state) = webauthn()
            .await?
            .start_passkey_registration(
                self.id_as_uuid()?,
                &self.email,
                &display_name,
                Some(exclude_credentials),
            )
            .to_internal_error()?;

        // resident keys are "discouraged" for passkeys, but required for 1fa login.
        // Unfortunately I want to use my titan keys for 2fa so we're making things more complicated by not requiring RKs.
        if let Some(selection) = options.public_key.authenticator_selection.as_mut() {
            selection.resident_key = Some(ResidentKeyRequirement::Preferred);
        }

        let token = PasskeyChallenge {
            account_id: Some(self.id.clone()),
            state: PasskeyChallengeState::Registration(state),
        }
        .save()
        .await?;

        Ok((token, serde_json::to_value(options).to_internal_error()?))
    }

    /// Finish registering a new passkey
    pub async fn finish_passkey_registration(
        &mut self,
        db: &Database,
        token: &str,
        name: String,
        credential: serde_json::Value,
    ) -> Result<PasskeyCredential> {
        let challenge = PasskeyChallenge::take(token).await?;
        let PasskeyChallengeState::Registration(state) = challenge.state else {
            return Err(create_error!(InvalidToken));
        };

        if challenge.account_id.as_deref() != Some(self.id.as_str()) {
            return Err(create_error!(InvalidToken));
        }

        let config = config().await;
        if self.mfa.passkeys.len() >= config.api.security.webauthn.max_passkeys {
            return Err(create_error!(TooManyPasskeys {
                max: config.api.security.webauthn.max_passkeys
            }));
        }

        let credential: RegisterPublicKeyCredential =
            serde_json::from_value(credential).map_err(|_| create_error!(InvalidToken))?;

        let passkey = webauthn()
            .await?
            .finish_passkey_registration(&credential, &state)
            .map_err(|_| create_error!(InvalidToken))?;

        if db
            .fetch_account_by_passkey(passkey.cred_id())
            .await?
            .is_some()
        {
            return Err(create_error!(PasskeyAlreadyRegistered));
        }

        let passkey = PasskeyCredential {
            id: ulid::Ulid::new().to_string(),
            credential_id: encode_credential_id(passkey.cred_id()),
            name,
            credential: passkey,
            // Technically props.rk isn't a trusted value,
            // but this is mostly a UI hint so it's fine
            discoverable: credential
                .extensions
                .cred_props
                .as_ref()
                .and_then(|props| props.rk)
                .unwrap_or(false),
            created_at: Timestamp::now_utc(),
            last_used: None,
        };

        self.mfa.passkeys.push(passkey.clone());
        self.save(db).await?;

        Ok(passkey)
    }

    /// Begin authenticating with one of this account's passkeys
    ///
    /// returns the challenge token and request options
    pub async fn start_passkey_authentication(&self) -> Result<(String, serde_json::Value)> {
        if self.mfa.passkeys.is_empty() {
            return Err(create_error!(DisallowedMFAMethod));
        }

        let passkeys: Vec<Passkey> = self
            .mfa
            .passkeys
            .iter()
            .map(|passkey| passkey.credential.clone())
            .collect();

        let (options, state) = webauthn()
            .await?
            .start_passkey_authentication(&passkeys)
            .to_internal_error()?;

        let token = PasskeyChallenge {
            account_id: Some(self.id.clone()),
            state: PasskeyChallengeState::Authentication(state),
        }
        .save()
        .await?;

        Ok((token, serde_json::to_value(options).to_internal_error()?))
    }

    /// Begin a passwordless login using any discoverable passkey
    ///
    /// returns the challenge token and request options
    pub async fn start_passkey_login() -> Result<(String, serde_json::Value)> {
        let (options, state) = webauthn()
            .await?
            .start_discoverable_authentication()
            .to_internal_error()?;

        let token = PasskeyChallenge {
            account_id: None,
            state: PasskeyChallengeState::Discoverable(state),
        }
        .save()
        .await?;

        Ok((token, serde_json::to_value(options).to_internal_error()?))
    }

    /// Resolve which account a passwordless login is for
    ///
    /// This does not verify the response. For that use [`Account::finish_passkey_login`]
    pub async fn identify_passkey_login(
        db: &Database,
        token: &str,
        response: serde_json::Value,
    ) -> Result<(Account, PasskeyLogin)> {
        let challenge = PasskeyChallenge::take(token).await?;
        let PasskeyChallengeState::Discoverable(state) = challenge.state else {
            return Err(create_error!(InvalidToken));
        };

        let response: PublicKeyCredential =
            serde_json::from_value(response).map_err(|_| create_error!(InvalidToken))?;

        let (user_id, credential_id) = webauthn()
            .await?
            .identify_discoverable_authentication(&response)
            .map_err(|_| create_error!(InvalidCredentials))?;

        let account = db
            .fetch_account_by_passkey(credential_id)
            .await?
            .ok_or_else(|| create_error!(InvalidCredentials))?;

        if account.id_as_uuid()? != user_id {
            return Err(create_error!(InvalidCredentials));
        }

        Ok((account, PasskeyLogin { state, response }))
    }

    /// Verify a passwordless login identified by [`Account::identify_passkey_login`]
    ///
    /// Model must be saved afterwards
    pub async fn finish_passkey_login(&mut self, db: &Database, login: PasskeyLogin) -> Result<()> {
        let passkey = self
            .mfa
            .find_passkey_mut(login.response.get_credential_id())
            .ok_or_else(|| create_error!(InvalidCredentials))?;

        let result = webauthn()
            .await?
            .finish_discoverable_authentication_v2(
                &login.response,
                login.state,
                &passkey.credential,
            )
            .map_err(|_| create_error!(InvalidCredentials))?;

        self.record_passkey_use(db, &result).await
    }

    /// Verify a passkey response against a challenge issued for this account
    ///
    /// This also saves a use of the passkey to the database, saving the account model
    async fn verify_passkey(
        &mut self,
        db: &Database,
        challenge: &str,
        response: serde_json::Value,
    ) -> Result<()> {
        let challenge = PasskeyChallenge::take(challenge).await?;
        let PasskeyChallengeState::Authentication(state) = challenge.state else {
            return Err(create_error!(InvalidToken));
        };

        if challenge.account_id.as_deref() != Some(self.id.as_str()) {
            return Err(create_error!(InvalidToken));
        }

        let response: PublicKeyCredential =
            serde_json::from_value(response).map_err(|_| create_error!(InvalidToken))?;

        let result = webauthn()
            .await?
            .finish_passkey_authentication(&response, &state)
            .map_err(|_| create_error!(InvalidToken))?;

        self.record_passkey_use(db, &result).await
    }

    /// Validate an MFA response
    pub async fn consume_mfa_response(
        &mut self,
        db: &Database,
        response: v0::MFAResponse,
        ticket: Option<&mut MFATicket>,
    ) -> Result<()> {
        let allowed_methods = self.mfa.get_methods();

        match response {
            v0::MFAResponse::Password { password } => {
                if allowed_methods.contains(&MFAMethod::Password) {
                    self.verify_password(&password)
                } else {
                    Err(create_error!(DisallowedMFAMethod))
                }
            }
            v0::MFAResponse::Totp { totp_code } => {
                if allowed_methods.contains(&MFAMethod::Totp) {
                    if let Totp::Enabled { .. } = &self.mfa.totp_token {
                        // Use TOTP code at generation if applicable
                        if let Some(ref ticket) = ticket {
                            if ticket.attempts >= 3 {
                                db.delete_ticket(&ticket.id).await?;
                                return Err(create_error!(InvalidToken));
                            }
                            if let Some(code) = &ticket.last_totp_code
                                && code == &totp_code
                            {
                                db.delete_ticket(&ticket.id).await?;
                                return Ok(());
                            }
                        }

                        // Otherwise read current TOTP token
                        if self.mfa.totp_token.generate_code()? == totp_code {
                            if let Some(ticket) = ticket {
                                db.delete_ticket(&ticket.id).await?;
                            }
                            Ok(())
                        } else {
                            if let Some(ticket) = ticket {
                                ticket.fail_attempt(db).await?;
                            }
                            Err(create_error!(InvalidToken))
                        }
                    } else {
                        unreachable!()
                    }
                } else {
                    Err(create_error!(DisallowedMFAMethod))
                }
            }
            v0::MFAResponse::Passkey {
                passkey_challenge,
                passkey_response,
            } => {
                if allowed_methods.contains(&MFAMethod::Passkey) {
                    if let Some(ref ticket) = ticket
                        && ticket.attempts >= 3
                    {
                        db.delete_ticket(&ticket.id).await?;
                        return Err(create_error!(InvalidToken));
                    }

                    match self
                        .verify_passkey(db, &passkey_challenge, passkey_response)
                        .await
                    {
                        Ok(()) => {
                            if let Some(ticket) = ticket {
                                db.delete_ticket(&ticket.id).await?;
                            }
                            Ok(())
                        }
                        Err(err) => {
                            if let Some(ticket) = ticket {
                                ticket.fail_attempt(db).await?;
                            }
                            Err(err)
                        }
                    }
                } else {
                    Err(create_error!(DisallowedMFAMethod))
                }
            }
            v0::MFAResponse::Recovery { recovery_code } => {
                if allowed_methods.contains(&MFAMethod::Recovery) {
                    if let Some(index) = self
                        .mfa
                        .recovery_codes
                        .iter()
                        .position(|x| x == &recovery_code)
                    {
                        self.mfa.recovery_codes.remove(index);
                        self.save(db).await
                    } else {
                        Err(create_error!(InvalidToken))
                    }
                } else {
                    Err(create_error!(DisallowedMFAMethod))
                }
            }
        }
    }

    /// Delete all sessions for an account
    pub async fn delete_all_sessions(
        &self,
        db: &Database,
        exclude_session_id: Option<String>,
    ) -> Result<()> {
        db.delete_all_sessions(&self.id, exclude_session_id.clone())
            .await?;

        // Create and push event
        EventV1::DeleteAllSessions {
            user_id: self.id.clone(),
            exclude_session_id,
        }
        .p(self.id.clone())
        .await;

        Ok(())
    }

    /// Disable an account
    pub async fn disable(&mut self, db: &Database) -> Result<()> {
        self.disabled = true;
        self.delete_all_sessions(db, None).await?;
        self.save(db).await
    }

    /// Schedule an account for deletion
    pub async fn schedule_deletion(&mut self, db: &Database) -> Result<()> {
        self.deletion = Some(DeletionInfo::Scheduled {
            after: Timestamp::now_utc()
                .checked_add(Duration::weeks(1))
                .unwrap(),
        });

        self.disable(db).await
    }

    /// Removes all information from the account and marks it as fully deleted
    pub async fn mark_deleted(&mut self, db: &Database) -> Result<()> {
        self.email = format!("Deleted User {}", &self.id);
        self.email_normalised = format!("Deleted User {}", &self.id);
        self.deletion = Some(DeletionInfo::Deleted);
        self.mfa.passkeys.clear();

        self.save(db).await?;

        Ok(())
    }
}
