use revolt_result::Result;

use crate::events::client::EventV1;
use crate::{Database, File};

auto_derived_partial!(
    /// Webhook
    pub struct Webhook {
        /// Webhook Id
        #[serde(rename = "_id")]
        pub id: String,

        /// The name of the webhook
        pub name: String,

        /// The avatar of the webhook
        #[serde(skip_serializing_if = "Option::is_none")]
        pub avatar: Option<File>,

        /// User that created this webhook
        pub creator_id: String,

        /// The channel this webhook belongs to
        pub channel_id: String,

        /// The permissions of the webhook
        pub permissions: u64,

        /// The private token for the webhook
        pub token: Option<String>,
    },
    "PartialWebhook"
);

auto_derived!(
    /// Optional fields on webhook object
    pub enum FieldsWebhook {
        Avatar,
    }
);

#[allow(clippy::derivable_impls)]
impl Default for Webhook {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
            avatar: None,
            creator_id: Default::default(),
            channel_id: Default::default(),
            permissions: Default::default(),
            token: Default::default(),
        }
    }
}

#[allow(clippy::disallowed_methods)]
impl Webhook {
    pub async fn create(&self, db: &Database) -> Result<()> {
        db.insert_webhook(self).await?;

        // Avoid leaking the token to people who receive the event
        let mut webhook = self.clone();
        webhook.token = None;

        EventV1::WebhookCreate(webhook.into())
            .p(self.channel_id.clone())
            .await;

        Ok(())
    }

    pub fn assert_token(&self, token: &str) -> Result<()> {
        if self.token.as_deref() == Some(token) {
            Ok(())
        } else {
            Err(create_error!(NotAuthenticated))
        }
    }

    pub async fn update(
        &mut self,
        db: &Database,
        mut partial: PartialWebhook,
        remove: Vec<FieldsWebhook>,
    ) -> Result<()> {
        for field in &remove {
            self.remove_field(field)
        }

        self.apply_options(partial.clone());

        db.update_webhook(&self.id, &partial, &remove).await?;

        partial.token = None; // Avoid leaking the token to people who receive the event

        EventV1::WebhookUpdate {
            id: self.id.clone(),
            channel_id: self.channel_id.clone(),
            data: partial.into(),
            remove: remove.into_iter().map(|v| v.into()).collect(),
        }
        .p(self.channel_id.clone())
        .await;

        Ok(())
    }

    pub fn remove_field(&mut self, field: &FieldsWebhook) {
        match field {
            FieldsWebhook::Avatar => self.avatar = None,
        }
    }

    pub async fn delete(&self, db: &Database) -> Result<()> {
        db.delete_webhook(&self.id).await?;

        EventV1::WebhookDelete {
            id: self.id.clone(),
            channel_id: self.channel_id.clone(),
        }
        .p(self.channel_id.clone())
        .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{FieldsWebhook, PartialWebhook, Webhook};

    #[async_std::test]
    async fn crud() {
        database_test!(|db| async move {
            let webhook_id = "webhook";
            let channel_id = "channel";

            let webhook = Webhook {
                id: webhook_id.to_string(),
                name: "Webhook Name".to_string(),
                channel_id: channel_id.to_string(),
                avatar: None,
                ..Default::default()
            };

            webhook.create(&db).await.unwrap();

            let mut updated_webhook = webhook.clone();
            updated_webhook
                .update(
                    &db,
                    PartialWebhook {
                        name: Some("New Name".to_string()),
                        ..Default::default()
                    },
                    vec![FieldsWebhook::Avatar],
                )
                .await
                .unwrap();

            let fetched_webhook = db.fetch_webhook(webhook_id).await.unwrap();
            let fetched_webhooks = db.fetch_webhooks_for_channel(channel_id).await.unwrap();

            assert_eq!(updated_webhook, fetched_webhook);
            assert_ne!(webhook, fetched_webhook);
            assert_eq!(1, fetched_webhooks.len());
            assert_eq!(fetched_webhook, fetched_webhooks[0]);

            webhook.delete(&db).await.unwrap();
            assert!(db.fetch_webhook(webhook_id).await.is_err());
            assert_eq!(
                0,
                db.fetch_webhooks_for_channel(channel_id)
                    .await
                    .unwrap()
                    .len()
            )
        });
    }
}
