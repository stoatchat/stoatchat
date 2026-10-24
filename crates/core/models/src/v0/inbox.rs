use crate::v0;
#[cfg(feature = "validator")]
use validator::Validate;

#[cfg(feature = "rocket")]
use rocket::FromForm;

auto_derived!(
    /// Inbox entry
    pub struct InboxEntry {
        /// Why this inbox entry exists
        pub entry_kind: InboxKind,
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

    /// Page of inbox entries
    pub struct InboxResponse {
        pub items: Vec<InboxEntry>,
        /// Pass as `before` to fetch the next page; absent when nothing older exists
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub pagination: Option<String>,
    }

    /// Options for fetching inbox entries
    #[cfg_attr(feature = "validator", derive(Validate))]
    #[cfg_attr(feature = "rocket", derive(FromForm))]
    pub struct OptionsFetchInbox {
        /// Maximum number of inbox entries to fetch (default 50)
        #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
        pub limit: Option<i64>,
        /// Message ID to fetch inbox entries older than; use pagination from the previous page
        pub before: Option<String>,
        /// Include direct mentions and replies
        pub mentions: Option<bool>,
        /// Include role mentions
        pub roles: Option<bool>,
        /// Include @everyone
        pub everyone: Option<bool>,
    }

    pub enum InboxKind { Mention, Role, Everyone }
);
