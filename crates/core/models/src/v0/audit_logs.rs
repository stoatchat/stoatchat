use crate::v0::{Member, PartialChannel, PartialMember, PartialRole, PartialServer, User};
use revolt_permissions::Override;

auto_derived!(
    /// Audit log entry
    pub struct AuditLogEntry {
        /// Unique ID
        #[serde(rename = "_id")]
        pub id: String,

        /// The server the entry happened in
        pub server: String,
        /// User provided reason
        pub reason: Option<String>,
        /// User who ran the action
        pub user: String,
        /// User this action is targetting
        pub target: Option<String>,
        /// The action ran
        pub action: AuditLogEntryAction,
    }

    /// Indivual action stored on the audit log
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
            permissions: Override,
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
            before: Vec<String>,
            after: Vec<String>,
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
        EmojiCreate {
            emoji: String,
            name: String,
        },
        EmojiDelete {
            emoji: String,
            name: String,
        },
    }

    /// Audit log query filters
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    #[cfg_attr(feature = "rocket", derive(rocket::FromForm))]
    pub struct OptionsAuditLogQuery {
        /// Filter by who ran the action
        #[cfg_attr(feature = "validator", validate(length(min = 26, max = 26)))]
        pub user: Option<String>,
        /// Filter by who the action is targetting
        #[cfg_attr(feature = "validator", validate(length(min = 26, max = 26)))]
        pub target: Option<String>,
        /// Filter by the action type
        pub r#type: Option<Vec<String>>,
        /// Entries before a certain entry id
        #[cfg_attr(feature = "validator", validate(length(min = 26, max = 26)))]
        pub before: Option<String>,
        /// Entries after a certain entry id
        #[cfg_attr(feature = "validator", validate(length(min = 26, max = 26)))]
        pub after: Option<String>,
        /// Maximum number of entries to fetch
        #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
        pub limit: Option<i64>,
    }

    /// Response containing the audit log entries and the users involved
    pub struct AuditLogQueryResponse {
        /// List of audit logs
        pub audit_logs: Vec<AuditLogEntry>,
        /// List of users
        pub users: Vec<User>,
        /// List of members
        pub members: Vec<Member>,
    }
);
