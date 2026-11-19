auto_derived!(
    /// Notification preference for a channel or server
    pub enum NotificationSettingPreference {
        AllMessages,
        Mentions,
        None,
    }

    /// Notification setting override for a channel or server
    pub struct NotificationSetting {
        /// Id of the channel the setting belongs to
        /// May be none if this setting is for an entire server
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub channel: Option<String>,
        /// Id of the server the setting belongs to
        /// May be none if this refers to a DM/Group channel
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub server: Option<String>,
        /// Id of the user the setting belongs to
        pub user: String,
        /// The actual preference
        pub preference: NotificationSettingPreference,
    }
);
