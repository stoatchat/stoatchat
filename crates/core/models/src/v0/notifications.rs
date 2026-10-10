use crate::v0;
#[cfg(feature = "validator")]
use validator::Validate;

#[cfg(feature = "rocket")]
use rocket::FromForm;

auto_derived!(
    /// Notification
    pub struct NotificationCenter {
        /// Why this notification exists
        pub kind: NotificationKind,
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
        /// The full message object
        pub message: v0::Message,
    }

    /// Page of notifications
    pub struct NotificationsResponse {
        pub notifications: Vec<NotificationCenter>,
        /// Pass as `before` to fetch the next page; absent when nothing older exists
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub next_before: Option<String>,
    }

    /// Options for fetching notifications
    #[cfg_attr(feature = "validator", derive(Validate))]
    #[cfg_attr(feature = "rocket", derive(FromForm))]
    pub struct OptionsFetchNotifications {
        /// Maximum number of notifications to fetch (default 50)
        #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
        pub limit: Option<i64>,
        /// Message ID to fetch notifications older than; use next_before from the previous page
        pub before: Option<String>,
        /// Include direct mentions and replies
        pub mentions: Option<bool>,
        /// Include role mentions
        pub roles: Option<bool>,
        /// Include @everyone
        pub everyone: Option<bool>,
    }

    pub enum NotificationKind { Mention, Role, Everyone }
);
