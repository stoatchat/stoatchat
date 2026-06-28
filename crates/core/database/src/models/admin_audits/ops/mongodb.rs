use tokio_stream::StreamExt;
use revolt_result::Result;

use crate::AdminAuditItem;
use crate::MongoDb;

use super::AbstractAdminAudits;

static COL: &str = "admin_audits";

#[async_trait]
impl AbstractAdminAudits for MongoDb {
    async fn admin_audit_insert(&self, audit: AdminAuditItem) -> Result<()> {
        query!(self, insert_one, COL, audit).map(|_| ())
    }

    async fn admin_audit_fetch(
        &self,
        before_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AdminAuditItem>> {
        if let Some(before) = before_id {
            Ok(self
                .col::<AdminAuditItem>(COL)
                .find(doc! {
                    "_id": { "$lt": before}
                })
                .sort(doc! {"_id": -1})
                .limit(limit)
                .await
                .map_err(|_| create_database_error!("find", COL))?
                .filter_map(|f| f.ok())
                .collect()
                .await)
        } else {
            Ok(self
                .col::<AdminAuditItem>(COL)
                .find(doc! {})
                .sort(doc! {"_id": -1})
                .limit(limit)
                .await
                .map_err(|_| create_database_error!("find", COL))?
                .filter_map(|f| f.ok())
                .collect()
                .await)
        }
    }
}
