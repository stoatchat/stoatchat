use crate::v0::MFATicket;

auto_derived!(
    /// # Change Email Data
    pub struct DataChangeEmail {
        /// Valid email address
        pub email: String,
        /// Current password
        pub current_password: String,
    }

    /// # Change Data
    pub struct DataChangePassword {
        /// New password
        pub password: String,
        /// Current password
        pub current_password: String,
    }

    /// # Account Deletion Token
    pub struct DataAccountDeletion {
        /// Deletion token
        pub token: String,
    }

    /// # Account Data
    pub struct DataCreateAccount {
        /// Valid email address
        pub email: String,
        /// Password
        pub password: String,
        /// Invite code
        pub invite: Option<String>,
        /// Captcha verification code
        pub captcha: Option<String>,
    }

    pub struct AccountInfo {
        #[serde(rename = "_id")]
        pub id: String,
        pub email: String,
    }

    /// # Password Reset
    pub struct DataPasswordReset {
        /// Reset token
        pub token: String,

        /// New password
        pub password: String,

        /// Whether to logout all sessions
        #[serde(default)]
        pub remove_sessions: bool,
    }

    /// # Resend Information
    pub struct DataResendVerification {
        /// Email associated with the account
        pub email: String,
        /// Captcha verification code
        pub captcha: Option<String>,
    }

    /// # Reset Information
    pub struct DataSendPasswordReset {
        /// Email associated with the account
        pub email: String,
        /// Captcha verification code
        pub captcha: Option<String>,
    }
);