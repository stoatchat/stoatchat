use revolt_result::Result;

use crate::ReferenceDb;
use crate::UserSettings;

use super::AbstractUserSettings;

#[async_trait]
impl AbstractUserSettings for ReferenceDb {
    /// Fetch a subset of user settings
    async fn fetch_user_settings(&'_ self, id: &str, filter: &'_ [String]) -> Result<UserSettings> {
        let user_settings = self.user_settings.lock().await;
        user_settings
            .get(id)
            .cloned()
            .map(|settings| {
                settings
                    .into_iter()
                    .filter(|(key, _)| filter.contains(key))
                    .collect()
            })
            .ok_or_else(|| create_error!(NotFound))
    }

    /// Update a subset of user settings
    async fn set_user_settings(&self, id: &str, settings: &UserSettings) -> Result<()> {
        let mut user_settings = self.user_settings.lock().await;
        if let Some(existing) = user_settings.get_mut(id) {
            existing.extend(settings.clone());
        } else {
            user_settings.insert(id.to_string(), settings.clone());
        }

        Ok(())
    }

    /// Delete all user settings
    async fn delete_user_settings(&self, id: &str) -> Result<()> {
        let mut user_settings = self.user_settings.lock().await;
        if user_settings.remove(id).is_some() {
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }
}
