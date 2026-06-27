use std::fmt::Display;

use elasticsearch::{
    BulkOperation, BulkParts, CreateParts, DeleteByQueryParts, DeleteParts, Elasticsearch,
    IndexParts, SearchParts,
    auth::Credentials,
    http::{
        response::Exception,
        transport::{SingleNodeConnectionPool, TransportBuilder},
    },
    indices::{IndicesCreateParts, IndicesDeleteParts},
};
use elasticsearch_dsl::{FieldSort, Query, Search, SearchResponse, Sort};
use linkify::{LinkFinder, LinkKind};
use revolt_database::{Database, Message, MessageWithUser, User};
use serde_json::{Map, Value, json, to_value};

pub use elasticsearch;

use crate::{AuthorType, MessageComponent, SearchTerms};

/// Elasticsearch errors
#[derive(Debug)]
pub enum Error {
    Http(elasticsearch::Error),
    Exception(Exception),
}

impl From<elasticsearch::Error> for Error {
    fn from(value: elasticsearch::Error) -> Self {
        Self::Http(value)
    }
}

impl From<Exception> for Error {
    fn from(value: Exception) -> Self {
        Self::Exception(value)
    }
}

impl std::error::Error for Error {}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(error) => write!(f, "Http error: {error}"),
            Error::Exception(exception) => write!(f, "Elasticsearch error: {exception:?}"),
        }
    }
}

/// Higher level elasticsearch API more fit for our specific usecase
#[derive(Debug, Clone)]
pub struct ElasticsearchClient {
    pub inner: Elasticsearch,
}

impl ElasticsearchClient {
    pub fn new(host: &str, port: u16, key: String) -> Self {
        let pool =
            SingleNodeConnectionPool::new(format!("{host}:{port}").as_str().try_into().unwrap());
        let transport = TransportBuilder::new(pool)
            .auth(Credentials::EncodedApiKey(key))
            .build()
            .unwrap();

        let inner = Elasticsearch::new(transport);

        Self { inner }
    }

    /// Delete messages index along with all documents
    pub async fn delete_indexes(&self) -> Result<(), Error> {
        let exception = self
            .inner
            .indices()
            .delete(IndicesDeleteParts::Index(&["messages"]))
            .send()
            .await?
            .exception()
            .await?;

        if let Some(exception) = exception {
            Err(exception.into())
        } else {
            Ok(())
        }
    }

    /// Create the messages index
    pub async fn setup_indexes(&self) -> Result<(), Error> {
        let exception = self
            .inner
            .indices()
            .create(IndicesCreateParts::Index("messages"))
            .body(json!({
                "mappings": {
                    "properties": {
                        "content": {"type": "text"},
                        "author": {"type": "keyword"},
                        "author_type": {"type": "keyword"},
                        "channel": {"type": "keyword"},
                        "mentions": {"type": "keyword"},
                        "role_mentions": {"type": "keyword"},
                        "pinned": {"type": "boolean"},
                        "embeds": {
                            "properties": {}
                        },
                        "attachments": {
                            "type": "nested",
                            "dynamic": false,
                            "properties": {
                                "metadata.type": {
                                    "type": "keyword"
                                }
                            }
                        },
                        "has_link": { "type": "boolean" },
                    }
                }
            }))
            .send()
            .await?
            .exception()
            .await?;

        if let Some(exception) = exception {
            Err(exception.into())
        } else {
            Ok(())
        }
    }

    /// Performs a search for messages, returns a vec of message ids
    pub async fn search(&self, terms: SearchTerms) -> Result<Vec<String>, Error> {
        let mut query = Query::bool().filter(Query::terms("channel", terms.channels));

        if let Some(content) = terms.filters.content {
            query = query.filter(Query::r#match("content", content))
        }

        if let Some(author) = terms.filters.author {
            query = query.filter(Query::terms("author", author))
        }

        if let Some(mentions) = terms.filters.mentions {
            query = query.filter(Query::terms("mentions", mentions))
        }

        if let Some(author_type) = terms.filters.author_type {
            query = query.filter(Query::terms("author_type", author_type))
        }

        if let Some(pinned) = terms.filters.pinned {
            if pinned {
                query = query.filter(Query::exists("pinned"))
            } else {
                query = query.filter(Query::bool().must_not(Query::exists("pinned")))
            }
        }

        if let Some(components) = terms.filters.components {
            let mut components_query = Query::bool();
            let mut attachments_query = Query::bool();

            for component in components {
                match component {
                    MessageComponent::Image => {
                        attachments_query = attachments_query
                            .should(Query::term("attachments.metadata.type", "Image"))
                    }
                    MessageComponent::Video => {
                        attachments_query = attachments_query
                            .should(Query::term("attachments.metadata.type", "Video"))
                    }
                    MessageComponent::Link => {
                        components_query = components_query.should(Query::exists("has_link"))
                    }
                    MessageComponent::File => {
                        attachments_query = attachments_query.should(Query::exists("attachments"))
                    }
                    MessageComponent::Embed => {
                        components_query = components_query.should(Query::exists("embeds"))
                    }
                };
            }

            query = query
                .filter(components_query.should(Query::nested("attachments", attachments_query)));
        }

        let search = Search::new()
            .query(query)
            .stats(false)
            .sort(Sort::FieldSort(
                FieldSort::new("_id".to_string()).order(terms.sort.unwrap_or_default().into()),
            ));

        let response = self
            .inner
            .search(SearchParts::Index(&["messages"]))
            .stored_fields(&[])
            .body(search)
            .size(terms.limit.unwrap_or(100) as i64)
            .from(terms.offset.unwrap_or(0) as i64)
            .send()
            .await?;

        if response.status_code().is_success() {
            let messages = response.json::<SearchResponse>().await?;
            Ok(messages.hits.hits.into_iter().map(|hit| hit.id).collect())
        } else {
            Err(response
                .exception()
                .await?
                .expect("No exception with error response.")
                .into())
        }
    }

    /// Creates a source for a message which can be stored and indexed into elasticsearch
    fn create_message_source(
        &self,
        _db: &Database,
        message: Message,
        author: Option<User>,
    ) -> Value {
        let mut map = Map::new();

        map.insert("channel".to_string(), Value::String(message.channel));

        map.insert("author".to_string(), Value::String(message.author));

        if let Some(content) = message.content {
            // Is there a better way to handle this? can elasticsearch index links itself?
            // Maybe in the future store the domains and be able to filter by that as well
            let mut finder = LinkFinder::new();
            finder.kinds(&[LinkKind::Url]);

            if finder.links(&content).next().is_some() {
                map.insert("has_link".to_string(), Value::Bool(true));
            }

            map.insert("content".to_string(), Value::String(content));
        }

        if let Some(attachments) = message.attachments {
            // TODO: fetch the file metadata from FileHash because of File.metadata deprecation
            // let metadata = attachment.as_hash(db).await.expect("Failed to fetch FileHash").metadata;

            map.insert(
                "attachments".to_string(),
                serde_json::to_value(attachments).unwrap(),
            );
        }

        if let Some(embeds) = message.embeds {
            map.insert("embeds".to_string(), serde_json::to_value(embeds).unwrap());
        }

        if let Some(mentions) = message.mentions {
            map.insert(
                "mentions".to_string(),
                serde_json::to_value(mentions).unwrap(),
            );
        }

        if let Some(role_mentions) = message.role_mentions {
            map.insert(
                "role_mentions".to_string(),
                serde_json::to_value(role_mentions).unwrap(),
            );
        }

        if let Some(pinned) = message.pinned {
            map.insert("pinned".to_string(), Value::Bool(pinned));
        }

        // This will turn bot author type to user author type if this is ran on a deleted message,
        // due to the author not existing anymore so fetching will fail, this is probably niche enough
        // to not really matter, might try fix in the future.
        map.insert(
            "author_type".to_string(),
            to_value(if message.webhook.is_some() {
                AuthorType::Webhook
            } else if author.is_some_and(|user| user.bot.is_some()) {
                AuthorType::Bot
            } else {
                AuthorType::User
            })
            .unwrap(),
        );

        Value::Object(map)
    }

    /// Bulk uploads and indexes messages to elasticsearch
    pub async fn bulk_index_messages(
        &self,
        db: &Database,
        messages: Vec<MessageWithUser>,
    ) -> Result<(), Error> {
        let mut ops = Vec::<BulkOperation<Value>>::new();

        for message in messages {
            let id = message.message.id.clone();
            let source = self.create_message_source(db, message.message, message.user);

            ops.push(BulkOperation::create(source).id(id).into());
        }

        let exception = self
            .inner
            .bulk(BulkParts::Index("messages"))
            .body(ops)
            .send()
            .await?
            .exception()
            .await?;

        if let Some(exception) = exception {
            Err(exception.into())
        } else {
            Ok(())
        }
    }

    /// Uploads and indexes a single message to elasticsearch
    pub async fn index_message(
        &self,
        db: &Database,
        message: Message,
        author: Option<User>,
    ) -> Result<(), Error> {
        let id = message.id.clone();
        let source = self.create_message_source(db, message, author);

        let exception = self
            .inner
            .create(CreateParts::IndexId("messages", &id))
            .body(source)
            .send()
            .await?
            .exception()
            .await?;

        if let Some(exception) = exception {
            Err(exception.into())
        } else {
            Ok(())
        }
    }

    /// Updates or upserts an existing message to elasticsearch
    pub async fn edit_message(
        &self,
        db: &Database,
        message: Message,
        author: Option<User>,
    ) -> Result<(), Error> {
        let id = message.id.clone();
        let source = self.create_message_source(db, message, author);

        let exception = self
            .inner
            .index(IndexParts::IndexId("messages", &id))
            .body(source)
            .send()
            .await?
            .exception()
            .await?;

        if let Some(exception) = exception {
            Err(exception.into())
        } else {
            Ok(())
        }
    }

    /// Deletes a message from elasticsearch
    pub async fn delete_message(&self, message_id: &str) -> Result<(), Error> {
        let exception = self
            .inner
            .delete(DeleteParts::IndexId("messages", message_id))
            .send()
            .await?
            .exception()
            .await?;

        if let Some(exception) = exception {
            Err(exception.into())
        } else {
            Ok(())
        }
    }

    /// Deletes all messages in a channel from elasticsearch
    pub async fn delete_channel(&self, channel_id: &str) -> Result<(), Error> {
        let exception = self
            .inner
            .delete_by_query(DeleteByQueryParts::Index(&["messages"]))
            .body(Search::new().query(Query::term("channel", channel_id)))
            .send()
            .await?
            .exception()
            .await?;

        if let Some(exception) = exception {
            Err(exception.into())
        } else {
            Ok(())
        }
    }
}
