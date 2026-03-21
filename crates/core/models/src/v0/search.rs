use iso8601_timestamp::Timestamp;
use std::collections::HashSet;

auto_derived!(
    pub struct DataChannelMessagesSearch {
        pub channel: Option<String>,
        pub server: Option<String>,
        pub offset: Option<u64>,
        pub limit: Option<u64>,
        pub filters: Option<DataChannelMessagesSearchFilters>,
        pub sort: Option<SortOrder>,
        pub include_users: Option<bool>,
    }

    pub struct DataChannelMessagesSearchFilters {
        pub content: Option<String>,
        pub author: Option<HashSet<String>>,
        pub mentions: Option<HashSet<String>>,
        pub role_mentions: Option<HashSet<String>>,
        pub before_date: Option<Timestamp>,
        pub after_date: Option<Timestamp>,
        pub author_type: Option<HashSet<AuthorType>>,
        pub pinned: Option<bool>,
        pub components: Option<HashSet<MessageComponent>>,
    }

    #[derive(Copy, Hash)]
    pub enum AuthorType {
        User,
        // Bot,
        Webhook,
    }

    #[derive(Copy, Hash)]
    pub enum MessageComponent {
        Image,
        Video,
        // Link,
        File,
        Embed,
    }

    #[derive(Copy, Default)]
    #[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
    pub enum SortOrder {
        Asc,
        #[default]
        Desc,
    }
);
