use crate::{if_false, Database};
use revolt_result::Result;

auto_derived_partial!(
    /// Account invite ticket
    pub struct AccountInvite {
        /// Invite code
        #[serde(rename = "_id")]
        pub id: String,
        /// Whether this invite ticket has been used
        #[serde(skip_serializing_if = "if_false", default)]
        pub used: bool,
        /// User ID that this invite was claimed by
        #[serde(skip_serializing_if = "Option::is_none")]
        pub claimed_by: Option<String>,
    },
    "PartialAccountInvite"
);

impl AccountInvite {
    /// Save model
    pub async fn save(&self, db: &Database) -> Result<()> {
        db.save_account_invite(self).await
    }
}
