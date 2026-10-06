#[cfg(feature = "validator")]
use validator::Validate;

#[cfg(feature = "rocket")]
use rocket::FromForm;

auto_derived!(
    /// Notification
    pub struct Notification {
        /// Unique Id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: String,
        /// Message ID
        pub message_id: String,
        /// Channel ID
        pub channel_id: String,
        /// Server ID
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "Option::is_none")
        )]
        pub server_id: Option<String>,
    }

    /// Options for fetching notifications
    #[cfg_attr(feature = "validator", derive(Validate))]
    #[cfg_attr(feature = "rocket", derive(FromForm))]
    pub struct OptionsFetchNotifications {
        /// Maximum number of notifications to fetch (default 50)
        #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
        pub limit: Option<i64>,
        /// Fetch notifications older than this ID
        pub before: Option<String>,
    }
);
