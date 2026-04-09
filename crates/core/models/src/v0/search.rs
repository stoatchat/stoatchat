use iso8601_timestamp::Timestamp;
use std::collections::HashSet;

auto_derived!(
    /// Options for searching messages in a server or channel
    pub struct DataChannelMessagesSearch {
        /// Channel to search in
        pub channel: Option<String>,
        /// Server to search in
        pub server: Option<String>,

        /// Filter options
        pub filters: Option<DataChannelMessagesSearchFilters>,

        /// What index to start the search at
        pub offset: Option<u64>,
        /// Max amount of messages to return
        pub limit: Option<u64>,
        /// Sort order
        pub sort: Option<SortOrder>,
    }

    /// Message search filters
    pub struct DataChannelMessagesSearchFilters {
        /// Message content
        pub content: Option<String>,
        /// Specific user
        pub author: Option<HashSet<String>>,

        /// Mentions a user
        pub mentions: Option<HashSet<String>>,
        /// Mentions a role
        pub role_mentions: Option<HashSet<String>>,

        /// Send before a specific date
        pub before_date: Option<Timestamp>,
        /// Sent after a specific date
        pub after_date: Option<Timestamp>,

        /// What type of user sent the message
        pub author_type: Option<HashSet<AuthorType>>,
        /// Whether the message is pinned or not
        pub pinned: Option<bool>,
        /// Require message to have a specific component type
        pub components: Option<HashSet<MessageComponent>>,
    }

    /// Message author type
    #[derive(Copy, Hash)]
    pub enum AuthorType {
        User,
        Bot,
        Webhook,
    }

    /// Message component
    #[derive(Copy, Hash)]
    pub enum MessageComponent {
        Image,
        Video,
        Link,
        File,
        Embed,
    }

    /// Message sort order
    #[derive(Copy, Default)]
    #[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
    pub enum SortOrder {
        Asc,
        #[default]
        Desc,
    }
);
