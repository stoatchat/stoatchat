use std::collections::HashMap;
use futures::future::try_join_all;
use indexmap::IndexSet;
use revolt_result::Result;
use std::time::SystemTime;
use ulid::Ulid;
use crate::{AppendMessage, FieldsMessage, Message, MessageQuery, PartialMessage, ReferenceDb};

use super::AbstractMessages;

#[async_trait]
impl AbstractMessages for ReferenceDb {
    /// Insert a new message into the database
    async fn insert_message(&self, message: &Message) -> Result<()> {
        let mut messages = self.messages.lock().await;
        if messages.contains_key(&message.id) {
            Err(create_database_error!("insert", "message"))
        } else {
            messages.insert(message.id.to_string(), message.clone());
            Ok(())
        }
    }

    /// Fetch a message by its id
    async fn fetch_message(&self, id: &str) -> Result<Message> {
        let messages = self.messages.lock().await;
        messages
            .get(id)
            .cloned()
            .ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch multiple messages by given query
    async fn fetch_messages(&self, query: MessageQuery) -> Result<Vec<Message>> {
        let messages = self.messages.lock().await;
        let matched_messages = messages
            .values()
            .filter(|message| {
                if let Some(channel) = &query.filter.channel {
                    if &message.channel != channel {
                        return false;
                    }
                }

                if let Some(author) = &query.filter.author {
                    if &message.author != author {
                        return false;
                    }
                }

                if let Some(query) = &query.filter.query {
                    if let Some(content) = &message.content {
                        if !content.to_lowercase().contains(query) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                if let Some(pinned) = query.filter.pinned {
                    if message.pinned.unwrap_or_default() == pinned {
                        return false
                    }
                }

                true
            })
            .cloned()
            .collect();

        // FIXME: sorting, etc (will be required for tests)

        Ok(matched_messages)

        /*
        // 2. Find query limit
        let limit = query.limit.unwrap_or(50);

        // 3. Apply message time period
        match query.time_period {
            MessageTimePeriod::Relative { nearby } => {
                // 3.1. Prepare filters
                let mut older_message_filter = filter.clone();
                let mut newer_message_filter = filter;

                older_message_filter.insert(
                    "_id",
                    doc! {
                        "$lt": &nearby
                    },
                );

                newer_message_filter.insert(
                    "_id",
                    doc! {
                        "$gte": &nearby
                    },
                );

                // 3.2. Execute in both directions
                let (a, b) = try_join!(
                    self.find_with_options::<_, Message>(
                        COL,
                        newer_message_filter,
                        FindOptions::builder()
                            .limit(limit / 2 + 1)
                            .sort(doc! {
                                "_id": 1_i32
                            })
                            .build(),
                    ),
                    self.find_with_options::<_, Message>(
                        COL,
                        older_message_filter,
                        FindOptions::builder()
                            .limit(limit / 2)
                            .sort(doc! {
                                "_id": -1_i32
                            })
                            .build(),
                    )
                )
                .map_err(|_| create_database_error!("find", COL))?;

                Ok([a, b].concat())
            }
            MessageTimePeriod::Absolute {
                before,
                after,
                sort,
            } => {
                // 3.1. Apply message ID filter
                if let Some(doc) = match (before, after) {
                    (Some(before), Some(after)) => Some(doc! {
                        "$lt": before,
                        "$gt": after
                    }),
                    (Some(before), _) => Some(doc! {
                        "$lt": before
                    }),
                    (_, Some(after)) => Some(doc! {
                        "$gt": after
                    }),
                    _ => None,
                } {
                    filter.insert("_id", doc);
                }

                // 3.2. Execute with given message sort
                self.find_with_options(
                    COL,
                    filter,
                    FindOptions::builder()
                        .limit(limit)
                        .sort(match sort.unwrap_or(MessageSort::Latest) {
                            // Sort by relevance, fallback to latest
                            MessageSort::Relevance => {
                                if is_search_query {
                                    doc! {
                                        "score": {
                                            "$meta": "textScore"
                                        }
                                    }
                                } else {
                                    doc! {
                                        "_id": -1_i32
                                    }
                                }
                            }
                            // Sort by latest first
                            MessageSort::Latest => doc! {
                                "_id": -1_i32
                            },
                            // Sort by oldest first
                            MessageSort::Oldest => doc! {
                                "_id": 1_i32
                            },
                        })
                        .build(),
                )
                .await
                .map_err(|_| create_database_error!("find", COL))
            }
        }*/
    }

    /// Fetch multiple messages by given IDs
    async fn fetch_messages_by_id(&self, ids: &[String]) -> Result<Vec<Message>> {
        try_join_all(ids.iter().map(|id| self.fetch_message(id))).await
    }

    /// Update a given message with new information
    async fn update_message(&self, id: &str, message: &PartialMessage, remove: Vec<FieldsMessage>) -> Result<()> {
        let mut messages = self.messages.lock().await;
        if let Some(message_data) = messages.get_mut(id) {
            message_data.apply_options(message.to_owned());

            for field in remove {
                #[allow(clippy::disallowed_methods)]
                message_data.remove_field(&field);
            }
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Append information to a given message
    async fn append_message(&self, id: &str, append: &AppendMessage) -> Result<()> {
        let mut messages = self.messages.lock().await;
        if let Some(message_data) = messages.get_mut(id) {
            if let Some(embeds) = &append.embeds {
                if !embeds.is_empty() {
                    if let Some(embeds_data) = &mut message_data.embeds {
                        embeds_data.extend(embeds.clone());
                    } else {
                        message_data.embeds = Some(embeds.clone());
                    }
                }
            }

            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Add a new reaction to a message
    async fn add_reaction(&self, id: &str, emoji: &str, user: &str) -> Result<()> {
        let mut messages = self.messages.lock().await;
        if let Some(message) = messages.get_mut(id) {
            if let Some(users) = message.reactions.get_mut(emoji) {
                users.insert(user.to_string());
            } else {
                message
                    .reactions
                    .insert(emoji.to_string(), IndexSet::from([user.to_string()]));
            }

            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Remove a reaction from a message
    async fn remove_reaction(&self, id: &str, emoji: &str, user: &str) -> Result<()> {
        let mut messages = self.messages.lock().await;
        if let Some(message) = messages.get_mut(id) {
            if let Some(users) = message.reactions.get_mut(emoji) {
                users.remove(&user.to_string());
            }

            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Remove reaction from a message
    async fn clear_reaction(&self, id: &str, emoji: &str) -> Result<()> {
        let mut messages = self.messages.lock().await;
        if let Some(message) = messages.get_mut(id) {
            message.reactions.remove(emoji);
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Delete a message from the database by its id
    async fn delete_message(&self, id: &str) -> Result<()> {
        let mut messages = self.messages.lock().await;
        if messages.remove(id).is_some() {
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Delete messages from a channel by their ids and corresponding channel id
    async fn delete_messages(&self, channel: &str, ids: &[String]) -> Result<()> {
        self.messages
            .lock()
            .await
            .retain(|id, message| message.channel != channel && !ids.contains(id));

        Ok(())
    }

    /// Delete all messages from a specific author in a list of channels from a certain ULID onwards
    async fn delete_messages_by_author_since(
        &self,
        channels: &[String],
        author: &str,
        since: SystemTime
    ) -> Result<HashMap<String, Vec<String>>> {
        let threshold_ulid = Ulid::from_datetime(since).to_string();
        let mut deleted_messages: HashMap<String, Vec<String>> = HashMap::new();

        self.messages
            .lock()
            .await
            .retain(|id, message| {
                let should_delete = message.author == author
                    && channels.contains(&message.channel)
                    && id.as_str() >= threshold_ulid.as_str();

                if should_delete {
                    deleted_messages
                        .entry(message.channel.clone())
                        .or_default()
                        .push(id.clone());
                }

                !should_delete
            });

        Ok(deleted_messages)
    }
}
