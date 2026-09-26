use revolt_result::Result;

use crate::{AuditLogEntry, AuditLogQuery};

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractAuditLogs: Sync + Send {
    /// Inserts an entry into the server's audit log
    async fn insert_audit_log_entry(&self, entry: &AuditLogEntry) -> Result<()>;

    /// Fetches a server's audit logs using the provided query options
    async fn get_server_audit_logs(
        &self,
        server: &str,
        query: AuditLogQuery,
    ) -> Result<Vec<AuditLogEntry>>;
}
