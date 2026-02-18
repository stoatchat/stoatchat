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
        Password { password: String },
        Recovery { recovery_code: String },
        Totp { totp_code: String },
    }

    #[derive(Default)]
    pub struct MultiFactorStatus {
        #[serde(skip_serializing_if = "crate::if_false")]
        pub email_otp: bool,
        #[serde(skip_serializing_if = "crate::if_false")]
        pub trusted_handover: bool,
        #[serde(skip_serializing_if = "crate::if_false")]
        pub email_mfa: bool,
        #[serde(skip_serializing_if = "crate::if_false")]
        pub totp_mfa: bool,
        #[serde(skip_serializing_if = "crate::if_false")]
        pub security_key_mfa: bool,
        #[serde(skip_serializing_if = "crate::if_false")]
        pub recovery_active: bool,
    }

    pub enum MFAMethod {
        Password,
        Recovery,
        Totp,
    }

    /// # Totp Secret
    pub struct ResponseTotpSecret {
        pub secret: String,
    }
);