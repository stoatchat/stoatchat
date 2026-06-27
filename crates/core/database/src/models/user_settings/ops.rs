use revolt_result::Result;

use crate::UserSettings;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractUserSettings: Sync + Send {
    /// Fetch a subset of user settings
    async fn fetch_user_settings(&'_ self, id: &str, filter: &'_ [String]) -> Result<UserSettings>;

    /// Update a subset of user settings
    async fn set_user_settings(&self, id: &str, settings: &UserSettings) -> Result<()>;

    /// Delete all user settings
    async fn delete_user_settings(&self, id: &str) -> Result<()>;
}
