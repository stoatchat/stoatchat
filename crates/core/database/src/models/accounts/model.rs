use iso8601_timestamp::{Duration, Timestamp};

use nanoid::nanoid;
use revolt_config::config;
use revolt_result::Result;
use serde_json::json;

use crate::{
    events::client::EventV1,
    util::{
        email::{email_templates, normalise_email, send_email},
        password::hash_password,
    },
    Database, MFATicket, Session,
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

        /// Security Key MFA token, enabled if present
        /// (2-Factor)
        // #[serde(skip_serializing_if = "Option::is_none")]
        // pub security_key_token: Option<String>,

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

impl MultiFactorAuthentication {
    // Check whether MFA is in-use
    pub fn is_active(&self) -> bool {
        matches!(self.totp_token, Totp::Enabled { .. })
    }

    // Check whether there are still usable recovery codes
    pub fn has_recovery(&self) -> bool {
        !self.recovery_codes.is_empty()
    }

    // Get available MFA methods
    pub fn get_methods(&self) -> Vec<MFAMethod> {
        if let Totp::Enabled { .. } = self.totp_token {
            let mut methods = vec![MFAMethod::Totp];

            if self.has_recovery() {
                methods.push(MFAMethod::Recovery);
            }

            methods
        } else {
            vec![MFAMethod::Password]
        }
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
        // Hash the user's password
        let password = hash_password(plaintext_password)?;

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
        let session = Session {
            id: ulid::Ulid::new().to_string(),
            token: nanoid!(64),

            user_id: self.id.clone(),
            name,

            last_seen: Timestamp::now_utc(),

            origin: None,
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

    /// Validate an MFA response
    pub async fn consume_mfa_response(
        &mut self,
        db: &Database,
        response: v0::MFAResponse,
        ticket: Option<MFATicket>,
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
                        if let Some(ticket) = ticket {
                            if let Some(code) = ticket.last_totp_code {
                                if code == totp_code {
                                    return Ok(());
                                }
                            }
                        }

                        // Otherwise read current TOTP token
                        if self.mfa.totp_token.generate_code()? == totp_code {
                            Ok(())
                        } else {
                            Err(create_error!(InvalidToken))
                        }
                    } else {
                        unreachable!()
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
        .private(self.id.clone())
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
}
