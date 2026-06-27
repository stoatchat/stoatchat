use revolt_models::v0::MessageSort;
use revolt_result::Result;

use crate::{Database, Message, MessageFilter, MessageQuery, MessageTimePeriod, Server, User};

auto_derived!(
    /// Snapshot of some content
    pub struct Snapshot {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,
        /// Report parent Id
        pub report_id: String,
        /// Snapshot of content
        pub content: SnapshotContent,
    }

    /// Enum to map into different models
    /// that can be saved in a snapshot
    #[serde(tag = "_type")]
    pub enum SnapshotContent {
        Message {
            /// Context before the message
            #[serde(rename = "_prior_context", default)]
            prior_context: Vec<Message>,

            /// Context after the message
            #[serde(rename = "_leading_context", default)]
            leading_context: Vec<Message>,

            /// Message
            #[serde(flatten)]
            message: Message,
        },
        Server(Server),
        User(User),
    }
);

impl SnapshotContent {
    /// Generate snapshot from a given message
    pub async fn generate_from_message(
        db: &Database,
        message: Message,
    ) -> Result<(SnapshotContent, Vec<String>)> {
        // Collect message attachments
        let files = message
            .attachments
            .as_ref()
            .map(|attachments| attachments.iter().map(|x| x.id.to_string()).collect())
            .unwrap_or_default();

        // Collect prior context
        let prior_context = db
            .fetch_messages(MessageQuery {
                filter: MessageFilter {
                    channel: Some(message.channel.to_string()),
                    ..Default::default()
                },
                limit: Some(15),
                time_period: MessageTimePeriod::Absolute {
                    before: Some(message.id.to_string()),
                    after: None,
                    sort: Some(MessageSort::Latest),
                },
            })
            .await?;

        // Collect leading context
        let leading_context = db
            .fetch_messages(MessageQuery {
                filter: MessageFilter {
                    channel: Some(message.channel.to_string()),
                    ..Default::default()
                },
                limit: Some(15),
                time_period: MessageTimePeriod::Absolute {
                    before: None,
                    after: Some(message.id.to_string()),
                    sort: Some(MessageSort::Oldest),
                },
            })
            .await?;

        Ok((
            SnapshotContent::Message {
                message,
                prior_context: prior_context.into_iter().collect(),
                leading_context: leading_context.into_iter().collect(),
            },
            files,
        ))
    }

    /// Generate snapshot from a given server
    pub fn generate_from_server(server: Server) -> Result<(SnapshotContent, Vec<String>)> {
        // Collect server's icon and banner
        let files = [&server.icon, &server.banner]
            .iter()
            .filter_map(|x| x.as_ref().map(|x| x.id.to_string()))
            .collect();

        Ok((SnapshotContent::Server(server), files))
    }

    /// Generate snapshot from a given user
    pub fn generate_from_user(user: User) -> Result<(SnapshotContent, Vec<String>)> {
        // Collect user's avatar and profile background
        let files = [
            user.avatar.as_ref(),
            user.profile
                .as_ref()
                .and_then(|profile| profile.background.as_ref()),
        ]
        .iter()
        .filter_map(|x| x.as_ref().map(|x| x.id.to_string()))
        .collect();

        Ok((SnapshotContent::User(user), files))
    }
}
