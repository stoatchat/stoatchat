use revolt_result::Result;

use crate::{NotificationSetting, NotificationSettingPreference, ReferenceDb};

use super::AbstractNotificationSettings;

#[async_trait]
impl AbstractNotificationSettings for ReferenceDb {
    /// Get overridden settings for a channel
    async fn get_notification_settings_for_channel(
        &self,
        user: &str,
        server: Option<&str>,
        channel: &str,
    ) -> Result<(Option<NotificationSetting>, Option<NotificationSetting>)> {
        let settings = self.notification_settings.lock().await;

        let server_setting = server.and_then(|server| {
            settings
                .get(&(user.to_string(), Some(server.to_string()), None))
                .cloned()
        });

        let channel_setting = settings
            .values()
            .find(|setting| setting.user == user && setting.channel.as_deref() == Some(channel))
            .cloned();

        Ok((server_setting, channel_setting))
    }

    /// Get the overridden settings for an entire server
    async fn get_notification_settings_for_server(
        &self,
        user: &str,
        server: &str,
    ) -> Result<Option<NotificationSetting>> {
        let settings = self.notification_settings.lock().await;
        Ok(settings
            .get(&(user.to_string(), Some(server.to_string()), None))
            .cloned())
    }

    /// Get all overridden settings for the servers the user is a part of
    async fn get_notification_settings_for_ready(
        &self,
        user: &str,
        server_ids: &[&str],
    ) -> Result<Vec<NotificationSetting>> {
        let settings = self.notification_settings.lock().await;
        Ok(settings
            .values()
            .filter(|setting| {
                setting.user == user
                    && setting
                        .server
                        .as_deref()
                        .is_none_or(|server| server_ids.contains(&server))
            })
            .cloned()
            .collect())
    }

    /// Set a notification override
    async fn set_notification_settings_for_channel(
        &self,
        user: &str,
        server: Option<&str>,
        channel: Option<&str>,
        preference: NotificationSettingPreference,
    ) -> Result<NotificationSetting> {
        let mut settings = self.notification_settings.lock().await;
        let key = (
            user.to_string(),
            server.map(ToString::to_string),
            channel.map(ToString::to_string),
        );

        let setting = NotificationSetting {
            channel: key.2.clone(),
            server: key.1.clone(),
            user: key.0.clone(),
            preference,
        };

        if setting.preference == NotificationSettingPreference::Mentions {
            settings.remove(&key);
        } else {
            settings.insert(key, setting.clone());
        }

        Ok(setting)
    }
}
