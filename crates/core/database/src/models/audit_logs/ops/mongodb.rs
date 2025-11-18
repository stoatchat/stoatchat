use mongodb::options::FindOptions;
use revolt_result::Result;

use crate::{AuditLogEntry, AuditLogQuery, MongoDb};

use super::AbstractAuditLogs;

static COL: &str = "audit_logs";

#[async_trait]
impl AbstractAuditLogs for MongoDb {
    async fn insert_audit_log_entry(&self, entry: &AuditLogEntry) -> Result<()> {
        query!(self, insert_one, COL, entry).map(|_| ())
    }

    async fn get_server_audit_logs(
        &self,
        server: &str,
        query: AuditLogQuery,
    ) -> Result<Vec<AuditLogEntry>> {
        let mut filter = doc! {
            "server": server
        };

        if let Some(user) = query.user {
            filter.insert("user", user);
        };

        if let Some(ty) = query.r#type {
            filter.insert("action.type", ty);
        };

        if let Some(doc) = match (query.before, query.after) {
            (Some(before), Some(after)) => Some(doc! {
                "$lt": before,
                "$gt": after
            }),
            (Some(before), _) => Some(doc! {
                "$lt": before
            }),
            (_, Some(after)) => Some(doc! {
                "$gt": after
            }),
            _ => None,
        } {
            filter.insert("_id", doc);
        };

        let limit = query.limit.unwrap_or(50);

        self.find_with_options(
            COL,
            filter,
            FindOptions::builder()
                .limit(limit)
                .sort(doc! { "_id": -1 })
                .build(),
        )
        .await
        .map_err(|_| create_database_error!("find", COL))
    }
}
