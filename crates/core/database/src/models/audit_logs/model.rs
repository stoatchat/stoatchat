use std::{collections::HashSet, time::Duration};

use iso8601_timestamp::Timestamp;
use revolt_config::config;
use ulid::Ulid;

use crate::{Database, PartialChannel, PartialMember, PartialRole, PartialServer, User};
use revolt_models::v0;
use revolt_permissions::OverrideField;
use revolt_result::Result;

auto_derived!(
    /// Audit log entry
    pub struct AuditLogEntry {
        /// Unique ID
        #[serde(rename = "_id")]
        pub id: String,

        /// When the audit log entry gets auto-deleted
        ///
        /// This is only stored in the database and not given to users.
        pub expires_at: Timestamp,

        /// The server the entry happened in
        pub server: String,
        /// User provided reason
        pub reason: Option<String>,
        /// User who ran the action
        pub user: String,
        /// The action ran
        pub action: AuditLogEntryAction,
    }

    /// Indivual audit log action
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
        MessagePin {
            message: String,
            author: String,
            channel: String,
        },
        MessageUnpin {
            message: String,
            author: String,
            channel: String,
        },
        BanCreate {
            user: String,
        },
        BanDelete {
            user: String,
        },
        ChannelCreate {
            channel: String,
            name: String,
        },
        ChannelEdit {
            channel: String,
            before: PartialChannel,
            after: PartialChannel,
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
            before: PartialMember,
            after: PartialMember,
        },
        MemberKick {
            user: String,
        },
        ServerEdit {
            before: PartialServer,
            after: PartialServer,
        },
        RoleEdit {
            role: String,
            before: PartialRole,
            after: PartialRole,
        },
        RoleCreate {
            role: String,
            name: String,
        },
        RoleDelete {
            role: String,
            name: String,
        },
        RolesReorder {
            positions: Vec<String>,
        },
        InviteCreate {
            invite: String,
            channel: String,
        },
        InviteDelete {
            invite: String,
            channel: String,
        },
        WebhookCreate {
            webhook: String,
            name: String,
            channel: String,
        },
        WebhookDelete {
            webhook: String,
            name: String,
            channel: String,
        },
        EmojiDelete {
            emoji: String,
            name: String,
        },
    }

    /// Audit Log Query
    pub struct AuditLogQuery {
        /// Filter by who ran the action
        pub user: Option<String>,
        /// Filter by the action type
        pub r#type: Option<Vec<String>>,
        /// Entries before a certain entry id
        pub before: Option<String>,
        /// Entries after a certain entry id
        pub after: Option<String>,
        /// Maximum number of entries to fetch
        pub limit: i64,
    }
);

impl AuditLogEntryAction {
    // TODO: migrate this to a rabbitmq queue to avoid spawning lots of tasks
    /// Generates an `AuditLogEntry` for the current action and inserts it into the database
    pub async fn insert<R: Into<Option<String>>>(
        self,
        db: &Database,
        server: String,
        reason: R,
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
            reason: reason.into(),
            user,
            action: self,
        };

        // running the insert inside a task can cause race conditions in the test so for now just dont use a task for tests for now
        // this will need to be redone for when we migrate to using rabbitmq here anyway.
        #[cfg(not(test))]
        async_std::task::spawn({
            let db = db.clone();
            let entry = entry.clone();

            async move { revolt_config::report_internal_error!(db.insert_audit_log_entry(&entry).await) }
        });

        #[cfg(test)]
        db.insert_audit_log_entry(&entry).await.unwrap();

        entry
    }
}

impl AuditLogEntry {
    /// Fetches the corrasponding users and members for each audit log entry
    pub async fn with_users(
        db: &Database,
        server_id: &str,
        user: &User,
        entries: &[Self],
    ) -> Result<(Vec<v0::User>, Vec<v0::Member>)> {
        let mut user_ids = HashSet::new();

        for entry in entries {
            user_ids.insert(entry.user.clone());

            match &entry.action {
                AuditLogEntryAction::MessageDelete { author, .. } => {
                    user_ids.insert(author.clone());
                }
                AuditLogEntryAction::BanCreate { user } => {
                    user_ids.insert(user.clone());
                }
                AuditLogEntryAction::BanDelete { user } => {
                    user_ids.insert(user.clone());
                }
                AuditLogEntryAction::ChannelCreate { .. } => {}
                AuditLogEntryAction::MemberEdit { user, .. } => {
                    user_ids.insert(user.clone());
                }
                AuditLogEntryAction::MemberKick { user } => {
                    user_ids.insert(user.clone());
                }
                AuditLogEntryAction::MessagePin { author, .. } => {
                    user_ids.insert(author.clone());
                }
                AuditLogEntryAction::MessageUnpin { author, .. } => {
                    user_ids.insert(author.clone());
                }
                AuditLogEntryAction::ServerEdit { .. } => {}
                AuditLogEntryAction::RoleEdit { .. } => {}
                AuditLogEntryAction::RoleCreate { .. } => {}
                AuditLogEntryAction::RoleDelete { .. } => {}
                AuditLogEntryAction::RolesReorder { .. } => {}
                AuditLogEntryAction::MessageBulkDelete { .. } => {}
                AuditLogEntryAction::ChannelEdit { .. } => {}
                AuditLogEntryAction::ChannelRolePermissionsEdit { .. } => {}
                AuditLogEntryAction::ChannelDelete { .. } => {}
                AuditLogEntryAction::InviteCreate { .. } => {}
                AuditLogEntryAction::InviteDelete { .. } => {}
                AuditLogEntryAction::WebhookCreate { .. } => {}
                AuditLogEntryAction::WebhookDelete { .. } => {}
                AuditLogEntryAction::EmojiDelete { .. } => {}
            };
        }

        let user_ids = user_ids.into_iter().collect::<Vec<_>>();

        let users = User::fetch_many_ids_as_mutuals(db, user, &user_ids).await?;
        let members = db
            .fetch_members(server_id, &user_ids)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();

        Ok((users, members))
    }
}
