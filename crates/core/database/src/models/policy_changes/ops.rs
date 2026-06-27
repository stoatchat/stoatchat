use revolt_result::Result;

use crate::PolicyChange;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractPolicyChange: Sync + Send {
    /// Fetch all policy changes
    async fn fetch_policy_changes(&self) -> Result<Vec<PolicyChange>>;

    /// Acknowledge policy changes
    async fn acknowledge_policy_changes(&self, user_id: &str) -> Result<()>;
}
