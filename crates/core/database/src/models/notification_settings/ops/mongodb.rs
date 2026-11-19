use bson::Document;
use mongodb::options::ReplaceOptions;
use revolt_result::Result;

use crate::{MongoDb, NotificationSetting, NotificationSettingPreference};

use super::AbstractNotificationSettings;

static COL: &str = "notification_settings";

#[async_trait]
impl AbstractNotificationSettings for MongoDb {
    /// Get overridden settings for a channel
    async fn get_notification_settings_for_channel(
        &self,
        user: &str,
        server: Option<&str>,
        channel: &str,
    ) -> Result<(Option<NotificationSetting>, Option<NotificationSetting>)> {
        let mut filters = vec![doc! {
            "user": user,
            "channel": channel
        }];

        if let Some(server) = server {
            filters.push(doc! {
                "user": user,
                "server": server,
                "channel": null
            });
        }

        let settings: Vec<NotificationSetting> = query!(
            self,
            find,
            COL,
            doc! {
                "$or": filters
            }
        )?;

        let (server_settings, channel_settings): (Vec<_>, Vec<_>) = settings
            .into_iter()
            .partition(|setting| setting.channel.is_none());

        Ok((
            server_settings.into_iter().next(),
            channel_settings.into_iter().next(),
        ))
    }

    /// Get the overridden settings for an entire server
    async fn get_notification_settings_for_server(
        &self,
        user: &str,
        server: &str,
    ) -> Result<Option<NotificationSetting>> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "user": user,
                "server": server,
                "channel": null
            }
        )
    }

    /// Get all overridden settings for the servers the user is a part of
    async fn get_notification_settings_for_ready(
        &self,
        user: &str,
        server_ids: &[&str],
    ) -> Result<Vec<NotificationSetting>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "user": user,
                "$or": [
                    { "server": { "$in": server_ids } },
                    { "server": null }
                ]
            }
        )
    }

    /// Set a notification override
    async fn set_notification_settings_for_channel(
        &self,
        user: &str,
        server: Option<&str>,
        channel: Option<&str>,
        preference: NotificationSettingPreference,
    ) -> Result<NotificationSetting> {
        let filter = doc! {
            "user": user,
            "server": server,
            "channel": channel
        };

        let setting = NotificationSetting {
            channel: channel.map(ToString::to_string),
            server: server.map(ToString::to_string),
            user: user.to_string(),
            preference,
        };

        if setting.preference == NotificationSettingPreference::Mentions {
            self.col::<Document>(COL)
                .delete_one(filter)
                .await
                .map_err(|_| create_database_error!("delete_one", COL))?;
        } else {
            self.col::<NotificationSetting>(COL)
                .replace_one(filter, &setting)
                .with_options(ReplaceOptions::builder().upsert(true).build())
                .await
                .map_err(|_| create_database_error!("replace_one", COL))?;
        }

        Ok(setting)
    }
}
