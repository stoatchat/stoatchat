use iso8601_timestamp::Timestamp;

use crate::v0::{MFAMethod, MFAResponse};

auto_derived!(
    pub struct Session {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,

        /// User Id
        pub user_id: String,

        /// Session token
        pub token: String,

        /// Display name
        pub name: String,

        /// When the session was last logged in
        pub last_seen: Timestamp,

        /// What is the session origin?
        /// This could be used to differentiate sessions that come from staging/test vs prod, etc.
        /// Authifier will set this to None by default. The application must fill it in.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub origin: Option<String>,

        /// Web Push subscription
        #[serde(skip_serializing_if = "Option::is_none")]
        pub subscription: Option<WebPushSubscription>,
    }

    /// Web Push subscription
    pub struct WebPushSubscription {
        pub endpoint: String,
        pub p256dh: String,
        pub auth: String,
    }

    /// # Edit Data
    pub struct DataEditSession {
        /// Session friendly name
        pub friendly_name: String,
    }

    pub struct SessionInfo {
        #[serde(rename = "_id")]
        pub id: String,
        pub name: String,
    }

    /// # Login Data
    #[serde(untagged)]
    pub enum DataLogin {
        Email {
            /// Email
            email: String,
            /// Password
            password: String,
            /// Friendly name used for the session
            friendly_name: Option<String>,
        },
        MFA {
            /// Unvalidated or authorised MFA ticket
            ///
            /// Used to resolve the correct account
            mfa_ticket: String,
            /// Valid MFA response
            ///
            /// This will take precedence over the `password` field where applicable
            mfa_response: Option<MFAResponse>,
            /// Friendly name used for the session
            friendly_name: Option<String>,
        },
    }

    #[serde(tag = "result")]
    pub enum ResponseLogin {
        Success(Session),
        MFA {
            ticket: String,
            allowed_methods: Vec<MFAMethod>,
        },
        Disabled {
            user_id: String,
        },
    }

);