use iso8601_timestamp::Timestamp;

auto_derived!(
    pub struct MFATicket {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,

        /// Account Id
        pub account_id: String,

        /// Unique Token
        pub token: String,

        /// Whether this ticket has been validated
        /// (can be used for account actions)
        pub validated: bool,

        /// Whether this ticket is authorised
        /// (can be used to log a user in)
        pub authorised: bool,

        /// TOTP code at time of ticket creation
        pub last_totp_code: Option<String>,
    }

    #[serde(untagged)]
    pub enum ResponseVerify {
        NoTicket,
        WithTicket {
            /// Authorised MFA ticket, can be used to log in
            ticket: MFATicket,
        },
    }

    /// MFA response
    #[serde(untagged)]
    pub enum MFAResponse {
        Password {
            password: String,
        },
        Recovery {
            recovery_code: String,
        },
        Totp {
            totp_code: String,
        },
        Passkey {
            passkey_challenge: String,
            passkey_response: serde_json::Value,
        },
    }

    #[derive(Default)]
    pub struct MultiFactorStatus {
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub email_otp: bool,
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub trusted_handover: bool,
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub email_mfa: bool,
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub totp_mfa: bool,
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub passkey_mfa: bool,
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub recovery_active: bool,
    }

    pub enum MFAMethod {
        Password,
        Recovery,
        Totp,
        Passkey,
    }

    /// # Totp Secret
    pub struct ResponseTotpSecret {
        pub secret: String,
    }

    /// # Passkey
    pub struct PasskeyInfo {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,
        /// User provided name for this passkey
        pub name: String,
        /// Whether this passkey can be used for passwordless login,
        /// as reported by the client at registration
        pub discoverable: bool,
        /// When this passkey was registered
        pub created_at: Timestamp,
        /// When this passkey was last used to authenticate
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_used: Option<Timestamp>,
    }

    /// # Passkey Registration Challenge
    pub struct ResponsePasskeyRegistration {
        /// Challenge token, send this back when finishing registration
        pub challenge: String,
        /// `CredentialCreationOptions` to pass to `navigator.credentials.create`
        pub options: serde_json::Value,
    }

    /// # Passkey Authentication Challenge
    pub struct ResponsePasskeyChallenge {
        /// Challenge token, send this back with the passkey response
        pub challenge: String,
        /// `CredentialRequestOptions` to pass to `navigator.credentials.get`
        pub options: serde_json::Value,
    }

    /// # Passkey Registration Data
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct DataRegisterPasskey {
        /// Challenge token from `POST /auth/mfa/passkeys`
        pub challenge: String,
        /// Name for this passkey
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub name: String,
        /// `PublicKeyCredential` returned by `navigator.credentials.create`
        pub credential: serde_json::Value,
    }

    /// # Passkey Edit Data
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct DataEditPasskey {
        /// New name for this passkey
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub name: String,
    }
);
