use std::collections::HashSet;

use iso8601_timestamp::Timestamp;
use revolt_models::v0;
use serde::Serialize;

/// Message author type
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Hash)]
pub enum AuthorType {
    User,
    Bot,
    Webhook,
}

/// Message component
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MessageComponent {
    Image,
    Video,
    Link,
    File,
    Embed,
}

/// Message search filters
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchFilters {
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

/// Message sort order
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

/// Options for searching messages in a server or channel
#[derive(Debug, Clone, PartialEq)]
pub struct SearchTerms {
    /// Channels to search in
    pub channels: Vec<String>,

    /// Filter options
    pub filters: SearchFilters,

    /// What index to start the search at
    pub offset: Option<u64>,
    /// Max amount of messages to return
    pub limit: Option<u64>,
    /// Sort order
    pub sort: Option<SortOrder>,
}

impl From<v0::AuthorType> for AuthorType {
    fn from(value: v0::AuthorType) -> Self {
        match value {
            v0::AuthorType::User => AuthorType::User,
            v0::AuthorType::Bot => AuthorType::Bot,
            v0::AuthorType::Webhook => AuthorType::Webhook,
        }
    }
}

impl From<v0::MessageComponent> for MessageComponent {
    fn from(value: v0::MessageComponent) -> Self {
        match value {
            v0::MessageComponent::Image => MessageComponent::Image,
            v0::MessageComponent::Video => MessageComponent::Video,
            v0::MessageComponent::Link => MessageComponent::Link,
            v0::MessageComponent::File => MessageComponent::File,
            v0::MessageComponent::Embed => MessageComponent::Embed,
        }
    }
}

impl From<v0::SortOrder> for SortOrder {
    fn from(value: v0::SortOrder) -> Self {
        match value {
            v0::SortOrder::Asc => SortOrder::Asc,
            v0::SortOrder::Desc => SortOrder::Desc,
        }
    }
}

impl From<v0::DataChannelMessagesSearchFilters> for SearchFilters {
    fn from(value: v0::DataChannelMessagesSearchFilters) -> Self {
        Self {
            content: value.content,
            author: value.author,
            mentions: value.mentions,
            role_mentions: value.role_mentions,
            before_date: value.before_date,
            after_date: value.after_date,
            author_type: value
                .author_type
                .map(|types| types.into_iter().map(Into::into).collect()),
            pinned: value.pinned,
            components: value
                .components
                .map(|types| types.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<SortOrder> for elasticsearch_dsl::SortOrder {
    fn from(value: SortOrder) -> Self {
        match value {
            SortOrder::Asc => elasticsearch_dsl::SortOrder::Asc,
            SortOrder::Desc => elasticsearch_dsl::SortOrder::Desc,
        }
    }
}
