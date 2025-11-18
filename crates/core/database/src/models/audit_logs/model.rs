use std::time::Duration;

use iso8601_timestamp::Timestamp;
use revolt_config::{config, report_internal_error};
use rocket::tokio;
use ulid::Ulid;

use crate::{
    Database, FieldsMember, FieldsRole, FieldsServer, PartialMember, PartialRole, PartialServer,
    FieldsChannel, PartialChannel,
};
use revolt_permissions::OverrideField;

auto_derived!(
    pub struct AuditLogEntry {
        #[serde(rename = "_id")]
        pub id: String,

        pub expires_at: Timestamp,

        pub server: String,
        pub reason: Option<String>,
        pub user: String,
        pub action: AuditLogEntryAction,
    }

    #[serde(tag = "type")]
    #[allow(clippy::large_enum_variant)]
    pub enum AuditLogEntryAction {
        MessageDelete {
            author: String,
            channel: String,
        },
        MessageBulkDelete {
            channel: String,
            count: usize,
        },
        BanCreate {
            user: String,
        },
        BanDelete {
            user: String,
        },
        ChannelCreate {
            channel: String,
        },
        ChannelEdit {
            channel: String,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            remove: Vec<FieldsChannel>,
            partial: PartialChannel,
        },
        ChannelRolePermissionsEdit {
            channel: String,
            role: String,
            permissions: OverrideField,
        },
        ChannelDelete {
            channel: String,
            name: String,
        },
        MemberEdit {
            user: String,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            remove: Vec<FieldsMember>,
            partial: PartialMember,
        },
        MemberKick {
            user: String,
        },
        ServerEdit {
            #[serde(skip_serializing_if = "Vec::is_empty")]
            remove: Vec<FieldsServer>,
            partial: PartialServer,
        },
        RoleEdit {
            role: String,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            remove: Vec<FieldsRole>,
            partial: PartialRole,
        },
        RoleCreate {
            role: String,
        },
        RoleDelete {
            role: String,
            name: String,
        },
        RolesReorder {
            positions: Vec<String>,
        },
        InviteDelete {
            invite: String,
            channel: String,
        },
        WebhookCreate {
            webhook: String,
            channel: String
        },
        EmojiDelete {
            emoji: String,
            name: String,
        },
    }

    pub struct AuditLogQuery {
        pub user: Option<String>,
        pub r#type: Option<String>,
        pub before: Option<String>,
        pub after: Option<String>,
        pub limit: Option<i64>,
    }
);

impl AuditLogEntryAction {
    // TODO: migrate this to a queue-esc system to avoid spawning lots of tasks
    pub async fn insert(
        self,
        db: &Database,
        server: String,
        reason: Option<String>,
        user: String,
    ) -> AuditLogEntry {
        let config = config().await;

        let id = Ulid::new();
        let expires_at = id
            .datetime()
            .checked_add(Duration::from_secs(config.api.audit_logs.expires_after))
            .unwrap()
            .into();

        let entry = AuditLogEntry {
            id: id.to_string(),
            expires_at,
            server,
            reason,
            user,
            action: self,
        };

        tokio::spawn({
            let db = db.clone();
            let entry = entry.clone();

            async move { report_internal_error!(db.insert_audit_log_entry(&entry).await) }
        });

        entry
    }
}
