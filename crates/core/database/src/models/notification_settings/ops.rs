use crate::{NotificationSetting, NotificationSettingPreference};
use revolt_result::Result;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractNotificationSettings: Send + Sync {
    /// Get overridden settings for a channel
    ///
    /// Omit server if this is a dm/group
    /// This returns the server settings first, then the channel override
    async fn get_notification_settings_for_channel(
        &self,
        user: &str,
        server: Option<&str>,
        channel: &str,
    ) -> Result<(Option<NotificationSetting>, Option<NotificationSetting>)>;

    /// Get the overridden settings for an entire server
    async fn get_notification_settings_for_server(
        &self,
        user: &str,
        server: &str,
    ) -> Result<Option<NotificationSetting>>;

    /// Get all overridden settings for the servers the user is a part of
    async fn get_notification_settings_for_ready(
        &self,
        user: &str,
        server_ids: &[&str],
    ) -> Result<Vec<NotificationSetting>>;

    /// Set a notification override.
    /// If set to Mentions, the record is deleted
    /// If channel belongs to a server, you must provide the server
    async fn set_notification_settings_for_channel(
        &self,
        user: &str,
        server: Option<&str>,
        channel: Option<&str>,
        preference: NotificationSettingPreference,
    ) -> Result<NotificationSetting>;
}
