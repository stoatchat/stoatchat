use iso8601_timestamp::Timestamp;
use revolt_result::Result;

use crate::PolicyChange;
use crate::ReferenceDb;

use super::AbstractPolicyChange;

#[async_trait]
impl AbstractPolicyChange for ReferenceDb {
    /// Fetch all policy changes
    async fn fetch_policy_changes(&self) -> Result<Vec<PolicyChange>> {
        let policy_changes = self.policy_changes.lock().await;
        Ok(policy_changes.values().cloned().collect())
    }

    /// Acknowledge policy changes
    async fn acknowledge_policy_changes(&self, user_id: &str) -> Result<()> {
        let mut users = self.users.lock().await;
        let user = users.get_mut(user_id).expect("user doesn't exist");
        user.last_acknowledged_policy_change = self
            .fetch_policy_changes()
            .await?
            .into_iter()
            .map(|policy| policy.created_time)
            .max()
            .unwrap_or(Timestamp::UNIX_EPOCH);

        Ok(())
    }
}
