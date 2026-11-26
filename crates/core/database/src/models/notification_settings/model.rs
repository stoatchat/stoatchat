use crate::Database;
use revolt_result::Result;

auto_derived!(
    pub enum NotificationSettingPreference {
        AllMessages,
        Mentions,
        None,
    }

    pub struct NotificationSetting {
        /// Id of the channel the setting belongs to
        /// May be none if this setting is for an entire server
        #[serde(skip_serializing_if = "Option::is_none")]
        pub channel: Option<String>,
        /// Id of the server the setting belongs to
        /// May be none if this refers to a DM/Group channel
        #[serde(skip_serializing_if = "Option::is_none")]
        pub server: Option<String>,
        /// Id of the user the setting belongs to
        pub user: String,
        /// The actual preference
        pub preference: NotificationSettingPreference,
    }
);

impl NotificationSetting {
    /// Get the notification settings for the user for the specified channel.
    /// If the channel belongs to a server, also provide the server id to apply the server preference underneath the channel.
    pub async fn get_for_channel(
        db: &Database,
        user: &str,
        server: Option<&str>,
        channel: &str,
    ) -> Result<Option<Self>> {
        let (server_setting, channel_setting) = db
            .get_notification_settings_for_channel(user, server, channel)
            .await?;

        Ok(channel_setting.or(server_setting))
    }

    /// Same as get_for_channel but only returns the preference, and defaults to Mentions if no settings are saved
    pub async fn get_preference_for_channel(
        db: &Database,
        user: &str,
        server: Option<&str>,
        channel: &str,
    ) -> Result<NotificationSettingPreference> {
        Ok(Self::get_for_channel(db, user, server, channel)
            .await?
            .map(|setting| setting.preference)
            .unwrap_or(NotificationSettingPreference::Mentions))
    }

    /// Get the preference for an entire server, defaulting to Mentions if no settings are saved
    pub async fn get_preference_for_server(
        db: &Database,
        user: &str,
        server: &str,
    ) -> Result<NotificationSettingPreference> {
        Ok(db
            .get_notification_settings_for_server(user, server)
            .await?
            .map(|setting| setting.preference)
            .unwrap_or(NotificationSettingPreference::Mentions))
    }

    /// Save this setting, removing the override if the preference is Mentions
    pub async fn save(&self, db: &Database) -> Result<Self> {
        db.set_notification_settings_for_channel(
            &self.user,
            self.server.as_deref(),
            self.channel.as_deref(),
            self.preference.clone(),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use lapin::{ExchangeKind, options::ExchangeDeclareOptions, types::FieldTable};

    use crate::{NotificationSetting, NotificationSettingPreference};

    #[tokio::test]
    async fn crud() {
        database_test!(|db| async move {
            // Creates the collection + unique index on mongo
            db.migrate_database().await.unwrap();

            // Nothing saved defaults to Mentions
            assert_eq!(
                NotificationSetting::get_preference_for_channel(&db, "user", Some("server"), "channel")
                    .await
                    .unwrap(),
                NotificationSettingPreference::Mentions
            );

            // Server preference applies to channels within it
            NotificationSetting {
                channel: None,
                server: Some("server".to_string()),
                user: "user".to_string(),
                preference: NotificationSettingPreference::None,
            }
            .save(&db)
            .await
            .unwrap();

            assert_eq!(
                NotificationSetting::get_preference_for_server(&db, "user", "server")
                    .await
                    .unwrap(),
                NotificationSettingPreference::None
            );
            assert_eq!(
                NotificationSetting::get_preference_for_channel(&db, "user", Some("server"), "channel")
                    .await
                    .unwrap(),
                NotificationSettingPreference::None
            );

            // Channel override wins over the server preference, saving twice must not duplicate
            for _ in 0..2 {
                NotificationSetting {
                    channel: Some("channel".to_string()),
                    server: Some("server".to_string()),
                    user: "user".to_string(),
                    preference: NotificationSettingPreference::AllMessages,
                }
                .save(&db)
                .await
                .unwrap();
            }

            let (server_setting, channel_setting) = db
                .get_notification_settings_for_channel("user", Some("server"), "channel")
                .await
                .unwrap();
            assert_eq!(server_setting.unwrap().preference, NotificationSettingPreference::None);
            assert_eq!(
                channel_setting.unwrap().preference,
                NotificationSettingPreference::AllMessages
            );

            // Other channels in the server still use the server preference
            assert_eq!(
                NotificationSetting::get_preference_for_channel(&db, "user", Some("server"), "other")
                    .await
                    .unwrap(),
                NotificationSettingPreference::None
            );

            // DM setting, plus noise from another server and another user
            for (user, server, channel) in [
                ("user", None, Some("dm")),
                ("user", Some("server2"), None),
                ("other_user", Some("server"), None),
            ] {
                NotificationSetting {
                    channel: channel.map(ToString::to_string),
                    server: server.map(ToString::to_string),
                    user: user.to_string(),
                    preference: NotificationSettingPreference::AllMessages,
                }
                .save(&db)
                .await
                .unwrap();
            }

            assert_eq!(
                NotificationSetting::get_preference_for_channel(&db, "user", None, "dm")
                    .await
                    .unwrap(),
                NotificationSettingPreference::AllMessages
            );

            let mut ready = db
                .get_notification_settings_for_ready("user", &["server"])
                .await
                .unwrap();
            ready.sort_by_key(|s| s.channel.clone());
            assert_eq!(ready.len(), 3);
            assert_eq!(ready[0].channel, None);
            assert_eq!(ready[1].channel.as_deref(), Some("channel"));
            assert_eq!(ready[2].channel.as_deref(), Some("dm"));

            // Setting Mentions removes the override
            NotificationSetting {
                channel: Some("channel".to_string()),
                server: Some("server".to_string()),
                user: "user".to_string(),
                preference: NotificationSettingPreference::Mentions,
            }
            .save(&db)
            .await
            .unwrap();

            let (_, channel_setting) = db
                .get_notification_settings_for_channel("user", Some("server"), "channel")
                .await
                .unwrap();
            assert!(channel_setting.is_none());
            assert_eq!(
                db.get_notification_settings_for_ready("user", &["server"])
                    .await
                    .unwrap()
                    .len(),
                2
            );
        });
    }
}
