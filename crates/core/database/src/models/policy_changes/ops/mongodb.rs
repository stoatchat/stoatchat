use bson::to_bson;
use iso8601_timestamp::Timestamp;
use revolt_result::Result;

use crate::MongoDb;
use crate::PolicyChange;
use crate::User;

use super::AbstractPolicyChange;

static COL: &str = "policy_changes";

#[async_trait]
impl AbstractPolicyChange for MongoDb {
    /// Fetch all policy changes
    async fn fetch_policy_changes(&self) -> Result<Vec<PolicyChange>> {
        query!(self, find, COL, doc! {})
    }

    /// Acknowledge policy changes
    async fn acknowledge_policy_changes(&self, user_id: &str) -> Result<()> {
        let latest_policy = self
            .fetch_policy_changes()
            .await?
            .into_iter()
            .map(|policy| policy.created_time)
            .max()
            .unwrap_or(Timestamp::UNIX_EPOCH);

        self.col::<User>("users")
            .update_one(
                doc! {
                    "_id": user_id
                },
                doc! {
                    "$set": {
                        "last_acknowledged_policy_change": to_bson(&latest_policy)
                            .map_err(|_| create_database_error!("to_bson", "timestamp"))?
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }
}
