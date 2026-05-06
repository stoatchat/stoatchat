use iso8601_timestamp::Timestamp;

use crate::{events::client::EventV1, Database};
use revolt_result::Result;

auto_derived_partial!(
    /// Session information
    pub struct Session {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,

        /// User Id
        pub user_id: String,

        /// Session token
        pub token: String,

        /// Display name
        pub name: String,

        /// When the session was last logged in
        pub last_seen: Timestamp,

        /// What is the session origin?
        /// This could be used to differentiate sessions that come from staging/test vs prod, etc.
        /// Authifier will set this to None by default. The application must fill it in.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub origin: Option<String>,

        /// Web Push subscription
        #[serde(skip_serializing_if = "Option::is_none")]
        pub subscription: Option<WebPushSubscription>,
    },
    "PartialSession"
);

auto_derived!(
    /// Web Push subscription
    pub struct WebPushSubscription {
        pub endpoint: String,
        pub p256dh: String,
        pub auth: String,
    }
);

impl Session {
    /// Save model
    pub async fn save(&self, db: &Database) -> Result<()> {
        db.save_session(self).await
    }

    /// Delete session
    pub async fn delete(self, db: &Database) -> Result<()> {
        // Delete from database
        db.delete_session(&self.id).await?;

        // Create and push event
        EventV1::DeleteSession {
            user_id: self.user_id.clone(),
            session_id: self.id,
        }
        .private(self.user_id)
        .await;

        Ok(())
    }
}
