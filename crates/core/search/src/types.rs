use std::collections::HashSet;

use iso8601_timestamp::Timestamp;
use revolt_database::{File, Metadata};
use revolt_models::v0;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Hash)]
pub enum AuthorType {
    User,
    // Bot,
    Webhook,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MessageComponent {
    Image,
    Video,
    // Link,
    File,
    Embed,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchFilters {
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchTerms {
    pub channels: Vec<String>,
    pub filters: SearchFilters,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
    pub sort: Option<SortOrder>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetadataFile {
    pub file: File,
    pub metadata: Metadata,
}

impl From<v0::AuthorType> for AuthorType {
    fn from(value: v0::AuthorType) -> Self {
        match value {
            v0::AuthorType::User => AuthorType::User,
            // v0::AuthorType::Bot => AuthorType::Bot,
            v0::AuthorType::Webhook => AuthorType::Webhook,
        }
    }
}

impl From<v0::MessageComponent> for MessageComponent {
    fn from(value: v0::MessageComponent) -> Self {
        match value {
            v0::MessageComponent::Image => MessageComponent::Image,
            v0::MessageComponent::Video => MessageComponent::Video,
            // v0::MessageComponent::Link => MessageComponent::Link,
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
