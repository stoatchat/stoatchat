use revolt_result::Result;

use crate::{AuditLogEntry, AuditLogQuery, ReferenceDb};

use super::AbstractAuditLogs;

#[async_trait]
impl AbstractAuditLogs for ReferenceDb {
    async fn insert_audit_log_entry(&self, entry: &AuditLogEntry) -> Result<()> {
        self.audit_logs
            .lock()
            .await
            .insert(entry.id.clone(), entry.clone());

        Ok(())
    }

    async fn get_server_audit_logs(
        &self,
        server: &str,
        query: AuditLogQuery,
    ) -> Result<Vec<AuditLogEntry>> {
        let lock = self.audit_logs.lock().await;

        let logs = lock
            .values()
            .filter(|entry| {
                if entry.server != server {
                    return false;
                };

                if let Some(user) = &query.user {
                    if &entry.user != user {
                        return false;
                    }
                }

                if let Some(before) = &query.before {
                    if &entry.id > before {
                        return false;
                    };
                };

                if let Some(after) = &query.after {
                    if &entry.id < after {
                        return false;
                    };
                };

                if let Some(action_types) = &query.r#type {
                    let entry_type = serde_json::to_value(entry.action.clone())
                        .unwrap()
                        .as_object()
                        .unwrap()
                        .get("type")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string();

                    if !action_types.contains(&entry_type) {
                        return false;
                    }
                };

                true
            })
            .take(query.limit.unwrap_or(50) as usize)
            .cloned()
            .collect();

        Ok(logs)
    }
}
