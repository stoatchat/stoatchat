use std::borrow::Cow;

use revolt_permissions::{
    calculate_user_permissions, ChannelType, Override, PermissionQuery, PermissionValue,
    RelationshipStatus, DEFAULT_PERMISSION_DIRECT_MESSAGE,
};

use crate::{Channel, Database, Member, Role, Server, User};

/// Resolve a role's effective permission override, blending its class default (if any)
/// underneath its own explicit override.
///
/// Bits the role hasn't explicitly allowed or denied "inherit" from the class default,
/// live - this is read fresh on every resolution, never copied onto the role, so editing
/// a class default immediately changes every role in that class that hasn't explicitly
/// overridden the bit in question. Roles without a class are returned unchanged, so this
/// is a no-op for every role that existed before this feature.
fn resolve_role_base_override(role: &Role, server: &Server) -> Override {
    let role_override: Override = role.permissions.into();

    let Some(class) = role.class else {
        return role_override;
    };

    let class_override: Override = server.get_class_default(class).permissions.into();
    let role_touched_bits = role_override.allow | role_override.deny;

    Override {
        allow: (class_override.allow & !role_touched_bits) | role_override.allow,
        deny: (class_override.deny & !role_touched_bits) | role_override.deny,
    }
}

/// Same blend as [`resolve_role_base_override`], but for a role's override on one
/// specific channel - `explicit` is the role's existing entry in that channel's
/// `role_permissions` map, if it has one.
fn resolve_role_channel_override(
    role: &Role,
    server: &Server,
    channel_id: &str,
    explicit: Option<Override>,
) -> Override {
    let role_override = explicit.unwrap_or_default();

    let Some(class) = role.class else {
        return role_override;
    };

    let Some(class_override) = server
        .get_class_default(class)
        .channel_overrides
        .get(channel_id)
        .copied()
        .map(Override::from)
    else {
        return role_override;
    };

    let role_touched_bits = role_override.allow | role_override.deny;

    Override {
        allow: (class_override.allow & !role_touched_bits) | role_override.allow,
        deny: (class_override.deny & !role_touched_bits) | role_override.deny,
    }
}

/// Permissions calculator
#[derive(Clone)]
pub struct DatabasePermissionQuery<'a> {
    #[allow(dead_code)]
    database: &'a Database,

    perspective: &'a User,
    user: Option<Cow<'a, User>>,
    channel: Option<Cow<'a, Channel>>,
    server: Option<Cow<'a, Server>>,
    member: Option<Cow<'a, Member>>,

    // flag_known_relationship: Option<&'a RelationshipStatus>,
    cached_user_permission: Option<PermissionValue>,
    cached_mutual_connection: Option<bool>,
    cached_permission: Option<u64>,
}

#[async_trait]
impl PermissionQuery for DatabasePermissionQuery<'_> {
    // * For calculating user permission

    /// Is our perspective user privileged?
    async fn are_we_privileged(&mut self) -> bool {
        self.perspective.privileged
    }

    /// Is our perspective user a bot?
    async fn are_we_a_bot(&mut self) -> bool {
        self.perspective.bot.is_some()
    }

    /// Is our perspective user and the currently selected user the same?
    async fn are_the_users_same(&mut self) -> bool {
        if let Some(other_user) = &self.user {
            self.perspective.id == other_user.id
        } else {
            false
        }
    }

    /// Get the relationship with have with the currently selected user
    async fn user_relationship(&mut self) -> RelationshipStatus {
        if let Some(other_user) = &self.user {
            if self.perspective.id == other_user.id {
                return RelationshipStatus::User;
            } else if let Some(bot) = &other_user.bot {
                // For the purposes of permissions checks,
                // assume owner is the same as bot
                if self.perspective.id == bot.owner {
                    return RelationshipStatus::User;
                }
            }

            if let Some(relations) = &self.perspective.relations {
                for entry in relations {
                    if entry.id == other_user.id {
                        return match entry.status {
                            crate::RelationshipStatus::None => RelationshipStatus::None,
                            crate::RelationshipStatus::User => RelationshipStatus::User,
                            crate::RelationshipStatus::Friend => RelationshipStatus::Friend,
                            crate::RelationshipStatus::Outgoing => RelationshipStatus::Outgoing,
                            crate::RelationshipStatus::Incoming => RelationshipStatus::Incoming,
                            crate::RelationshipStatus::Blocked => RelationshipStatus::Blocked,
                            crate::RelationshipStatus::BlockedOther => {
                                RelationshipStatus::BlockedOther
                            }
                        };
                    }
                }
            }
        }

        RelationshipStatus::None
    }

    /// Whether the currently selected user is a bot
    async fn user_is_bot(&mut self) -> bool {
        if let Some(other_user) = &self.user {
            other_user.bot.is_some()
        } else {
            false
        }
    }

    /// Do we have a mutual connection with the currently selected user?
    async fn have_mutual_connection(&mut self) -> bool {
        if let Some(value) = self.cached_mutual_connection {
            value
        } else if let Some(user) = &self.user {
            let value = self
                .perspective
                .has_mutual_connection(self.database, &user.id)
                .await
                .unwrap_or_default();

            self.cached_mutual_connection = Some(value);
            value
        } else {
            false
        }
    }

    // * For calculating server permission

    /// Is our perspective user the server's owner?
    async fn are_we_server_owner(&mut self) -> bool {
        if let Some(server) = &self.server {
            server.owner == self.perspective.id
        } else {
            false
        }
    }

    /// Is our perspective user a member of the server?
    async fn are_we_a_member(&mut self) -> bool {
        if let Some(server) = &self.server {
            if self.member.is_some() {
                true
            } else if let Ok(member) = self
                .database
                .fetch_member(&server.id, &self.perspective.id)
                .await
            {
                self.member = Some(Cow::Owned(member));
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get default server permission
    async fn get_default_server_permissions(&mut self) -> u64 {
        if let Some(server) = &self.server {
            server.default_permissions as u64
        } else {
            0
        }
    }

    /// Get the ordered role overrides (from lowest to highest) for this member in this server
    async fn get_our_server_role_overrides(&mut self) -> Vec<Override> {
        if let Some(server) = &self.server {
            let member_roles = self
                .member
                .as_ref()
                .map(|member| member.roles.clone())
                .unwrap_or_default();

            let mut roles = server
                .roles
                .iter()
                .filter(|(id, _)| member_roles.contains(id))
                .map(|(_, role)| {
                    let v = resolve_role_base_override(role, server);
                    (role.rank, v)
                })
                .collect::<Vec<(i64, Override)>>();

            roles.sort_by(|a, b| b.0.cmp(&a.0));
            roles.into_iter().map(|(_, v)| v).collect()
        } else {
            vec![]
        }
    }

    /// Is our perspective user timed out on this server?
    async fn are_we_timed_out(&mut self) -> bool {
        if let Some(member) = &self.member {
            member.in_timeout()
        } else {
            false
        }
    }

    async fn do_we_have_publish_overwrites(&mut self) -> bool {
        if let Some(member) = &self.member {
            member.can_publish
        } else {
            true
        }
    }

    async fn do_we_have_receive_overwrites(&mut self) -> bool {
        if let Some(member) = &self.member {
            member.can_receive
        } else {
            true
        }
    }

    // * For calculating channel permission

    /// Get the type of the channel
    #[allow(deprecated)]
    async fn get_channel_type(&mut self) -> ChannelType {
        if let Some(channel) = &self.channel {
            match channel {
                Cow::Borrowed(Channel::DirectMessage { .. })
                | Cow::Owned(Channel::DirectMessage { .. }) => ChannelType::DirectMessage,
                Cow::Borrowed(Channel::Group { .. }) | Cow::Owned(Channel::Group { .. }) => {
                    ChannelType::Group
                }
                Cow::Borrowed(Channel::SavedMessages { .. })
                | Cow::Owned(Channel::SavedMessages { .. }) => ChannelType::SavedMessages,
                Cow::Borrowed(Channel::TextChannel { .. })
                | Cow::Owned(Channel::TextChannel { .. }) => ChannelType::ServerChannel,
            }
        } else {
            ChannelType::Unknown
        }
    }

    /// Get the default channel permissions
    /// Group channel defaults should be mapped to an allow-only override
    async fn get_default_channel_permissions(&mut self) -> Override {
        if let Some(channel) = &self.channel {
            match channel {
                Cow::Borrowed(Channel::Group { permissions, .. })
                | Cow::Owned(Channel::Group { permissions, .. }) => Override {
                    allow: permissions.unwrap_or(*DEFAULT_PERMISSION_DIRECT_MESSAGE as i64) as u64,
                    deny: 0,
                },
                Cow::Borrowed(Channel::TextChannel {
                    default_permissions,
                    ..
                })
                | Cow::Owned(Channel::TextChannel {
                    default_permissions,
                    ..
                }) => default_permissions.unwrap_or_default().into(),
                _ => Default::default(),
            }
        } else {
            Default::default()
        }
    }

    /// Get the ordered role overrides (from lowest to highest) for this member in this channel
    async fn get_our_channel_role_overrides(&mut self) -> Vec<Override> {
        if let Some(channel) = &self.channel {
            match channel {
                Cow::Borrowed(Channel::TextChannel {
                    id: channel_id,
                    role_permissions,
                    ..
                })
                | Cow::Owned(Channel::TextChannel {
                    id: channel_id,
                    role_permissions,
                    ..
                }) => {
                    if let Some(server) = &self.server {
                        let member_roles = self
                            .member
                            .as_ref()
                            .map(|member| member.roles.clone())
                            .unwrap_or_default();

                        // Iterate every role the member holds, not just ones with an
                        // explicit entry in `role_permissions` - a classed role with no
                        // explicit override on this channel still needs its class's
                        // channel template considered.
                        let mut roles = server
                            .roles
                            .iter()
                            .filter(|(id, _)| member_roles.contains(id))
                            .map(|(id, role)| {
                                let explicit = role_permissions.get(id).map(|p| (*p).into());
                                let v = resolve_role_channel_override(
                                    role, server, channel_id, explicit,
                                );
                                (role.rank, v)
                            })
                            .collect::<Vec<(i64, Override)>>();

                        roles.sort_by(|a, b| b.0.cmp(&a.0));
                        roles.into_iter().map(|(_, v)| v).collect()
                    } else {
                        vec![]
                    }
                }
                _ => vec![],
            }
        } else {
            vec![]
        }
    }

    /// Do we own this group or saved messages channel if it is one of those?
    async fn do_we_own_the_channel(&mut self) -> bool {
        if let Some(channel) = &self.channel {
            match channel {
                Cow::Borrowed(Channel::Group { owner, .. })
                | Cow::Owned(Channel::Group { owner, .. }) => owner == &self.perspective.id,
                Cow::Borrowed(Channel::SavedMessages { user, .. })
                | Cow::Owned(Channel::SavedMessages { user, .. }) => user == &self.perspective.id,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Are we a recipient of this channel?
    async fn are_we_part_of_the_channel(&mut self) -> bool {
        if let Some(
            Cow::Borrowed(Channel::DirectMessage { recipients, .. })
            | Cow::Owned(Channel::DirectMessage { recipients, .. })
            | Cow::Borrowed(Channel::Group { recipients, .. })
            | Cow::Owned(Channel::Group { recipients, .. }),
        ) = &self.channel
        {
            recipients.contains(&self.perspective.id)
        } else {
            false
        }
    }

    /// Set the current user as the recipient of this channel
    /// (this will only ever be called for DirectMessage channels, use unimplemented!() for other code paths)
    async fn set_recipient_as_user(&mut self) {
        if let Some(channel) = &self.channel {
            match channel {
                Cow::Borrowed(Channel::DirectMessage { recipients, .. })
                | Cow::Owned(Channel::DirectMessage { recipients, .. }) => {
                    let recipient_id = recipients
                        .iter()
                        .find(|recipient| recipient != &&self.perspective.id)
                        .expect("Missing recipient for DM");

                    if let Ok(user) = self.database.fetch_user(recipient_id).await {
                        self.user.replace(Cow::Owned(user));
                    }
                }
                _ => unimplemented!(),
            }
        }
    }

    /// Set the current server as the server owning this channel
    /// (this will only ever be called for server channels, use unimplemented!() for other code paths)
    async fn set_server_from_channel(&mut self) {
        if let Some(channel) = &self.channel {
            #[allow(deprecated)]
            match channel {
                Cow::Borrowed(Channel::TextChannel { server, .. })
                | Cow::Owned(Channel::TextChannel { server, .. }) => {
                    if let Some(known_server) =
                        // I'm not sure why I can't just pattern match both at once here?
                        // It throws some weird error and the provided fix doesn't work :/
                        if let Some(Cow::Borrowed(known_server)) = self.server {
                                Some(known_server)
                            } else if let Some(Cow::Owned(ref known_server)) = self.server {
                                Some(known_server)
                            } else {
                                None
                            }
                    {
                        if server == &known_server.id {
                            // Already cached, return early.
                            return;
                        }
                    }

                    if let Ok(server) = self.database.fetch_server(server).await {
                        self.server.replace(Cow::Owned(server));
                    }
                }
                _ => unimplemented!(),
            }
        }
    }
}

impl<'a> DatabasePermissionQuery<'a> {
    /// Create a new permission calculator
    pub fn new(database: &'a Database, perspective: &'a User) -> DatabasePermissionQuery<'a> {
        DatabasePermissionQuery {
            database,
            perspective,
            user: None,
            channel: None,
            server: None,
            member: None,

            cached_mutual_connection: None,
            cached_user_permission: None,
            cached_permission: None,
        }
    }

    /// Calculate the user permission value
    pub async fn calc_user(mut self) -> DatabasePermissionQuery<'a> {
        if self.cached_user_permission.is_some() {
            return self;
        }

        if self.user.is_none() {
            panic!("Expected `PermissionCalculator.user to exist.");
        }

        DatabasePermissionQuery {
            cached_user_permission: Some(calculate_user_permissions(&mut self).await),
            ..self
        }
    }

    /// Calculate the permission value
    pub async fn calc(self) -> DatabasePermissionQuery<'a> {
        if self.cached_permission.is_some() {
            return self;
        }

        self
    }

    /// Use user
    pub fn user(self, user: &'a User) -> DatabasePermissionQuery<'a> {
        DatabasePermissionQuery {
            user: Some(Cow::Borrowed(user)),
            ..self
        }
    }

    /// Use channel
    pub fn channel(self, channel: &'a Channel) -> DatabasePermissionQuery<'a> {
        DatabasePermissionQuery {
            channel: Some(Cow::Borrowed(channel)),
            ..self
        }
    }

    /// Use server
    pub fn server(self, server: &'a Server) -> DatabasePermissionQuery<'a> {
        DatabasePermissionQuery {
            server: Some(Cow::Borrowed(server)),
            ..self
        }
    }

    /// Use member
    pub fn member(self, member: &'a Member) -> DatabasePermissionQuery<'a> {
        DatabasePermissionQuery {
            member: Some(Cow::Borrowed(member)),
            ..self
        }
    }

    /// Access the underlying user
    pub fn user_ref(&self) -> &Option<Cow<User>> {
        &self.user
    }

    /// Access the underlying server
    pub fn channel_ref(&self) -> &Option<Cow<Channel>> {
        &self.channel
    }

    /// Access the underlying server
    pub fn server_ref(&self) -> &Option<Cow<Server>> {
        &self.server
    }

    /// Access the underlying member
    pub fn member_ref(&self) -> &Option<Cow<Member>> {
        &self.member
    }

    /// Get the known member's current ranking
    pub fn get_member_rank(&self) -> Option<i64> {
        self.member
            .as_ref()
            .map(|member| member.get_ranking(self.server.as_ref().unwrap()))
    }
}

/// Short-hand for creating a permission calculator
pub fn perms<'a>(database: &'a Database, perspective: &'a User) -> DatabasePermissionQuery<'a> {
    DatabasePermissionQuery::new(database, perspective)
}

#[cfg(test)]
mod resolve_override_tests {
    use std::collections::HashMap;

    use revolt_permissions::{ChannelPermission, OverrideField, RoleClass};

    use crate::{ClassDefault, Role, Server};

    use super::{resolve_role_base_override, resolve_role_channel_override};

    fn test_role(permissions: OverrideField, class: Option<RoleClass>) -> Role {
        Role {
            id: "role".into(),
            name: "Test Role".into(),
            permissions,
            colour: None,
            hoist: false,
            rank: 0,
            icon: None,
            class,
            max_message_length: None,
        }
    }

    fn test_server(class_defaults: HashMap<RoleClass, ClassDefault>) -> Server {
        Server {
            id: "server".into(),
            owner: "owner".into(),
            name: "Test Server".into(),
            description: None,
            channels: vec![],
            categories: None,
            system_messages: None,
            roles: HashMap::new(),
            default_permissions: 0,
            class_defaults,
            icon: None,
            banner: None,
            flags: None,
            nsfw: false,
            analytics: false,
            discoverable: false,
        }
    }

    #[test]
    fn role_without_class_is_unaffected_by_class_defaults() {
        let role = test_role(
            OverrideField {
                a: ChannelPermission::SendMessage as i64,
                d: 0,
            },
            None,
        );

        let server = test_server(HashMap::from([(
            RoleClass::Admin,
            ClassDefault::built_in(RoleClass::Admin),
        )]));

        let resolved = resolve_role_base_override(&role, &server);
        assert_eq!(resolved.allow, ChannelPermission::SendMessage as u64);
        assert_eq!(resolved.deny, 0);
    }

    #[test]
    fn classed_role_with_no_override_fully_inherits_class_default() {
        let role = test_role(OverrideField { a: 0, d: 0 }, Some(RoleClass::Member));
        let server = test_server(HashMap::new()); // uses ClassDefault::built_in fallback

        let resolved = resolve_role_base_override(&role, &server);
        let expected = ClassDefault::built_in(RoleClass::Member).permissions;
        assert_eq!(resolved.allow, expected.a as u64);
        assert_eq!(resolved.deny, expected.d as u64);
    }

    #[test]
    fn classed_role_explicit_bits_win_over_class_default() {
        // Class default allows SendMessage; role explicitly denies it - the role's
        // explicit choice must win even though it disagrees with the class.
        let mut class_defaults = HashMap::new();
        class_defaults.insert(
            RoleClass::Member,
            ClassDefault {
                permissions: OverrideField {
                    a: ChannelPermission::SendMessage as i64,
                    d: 0,
                },
                channel_overrides: HashMap::new(),
                max_message_length: None,
            },
        );

        let role = test_role(
            OverrideField {
                a: 0,
                d: ChannelPermission::SendMessage as i64,
            },
            Some(RoleClass::Member),
        );
        let server = test_server(class_defaults);

        let resolved = resolve_role_base_override(&role, &server);
        assert_eq!(resolved.allow & ChannelPermission::SendMessage as u64, 0);
        assert_eq!(
            resolved.deny & ChannelPermission::SendMessage as u64,
            ChannelPermission::SendMessage as u64
        );
    }

    #[test]
    fn classed_role_untouched_bits_still_inherit_alongside_explicit_override() {
        // Role explicitly grants UploadFiles (something the class default doesn't
        // mention) - the class's other bits (e.g. SendMessage) should still come
        // through untouched.
        let mut class_defaults = HashMap::new();
        class_defaults.insert(
            RoleClass::Member,
            ClassDefault {
                permissions: OverrideField {
                    a: ChannelPermission::SendMessage as i64,
                    d: 0,
                },
                channel_overrides: HashMap::new(),
                max_message_length: None,
            },
        );

        let role = test_role(
            OverrideField {
                a: ChannelPermission::UploadFiles as i64,
                d: 0,
            },
            Some(RoleClass::Member),
        );
        let server = test_server(class_defaults);

        let resolved = resolve_role_base_override(&role, &server);
        assert_eq!(
            resolved.allow & ChannelPermission::SendMessage as u64,
            ChannelPermission::SendMessage as u64
        );
        assert_eq!(
            resolved.allow & ChannelPermission::UploadFiles as u64,
            ChannelPermission::UploadFiles as u64
        );
    }

    #[test]
    fn live_link_class_default_change_is_picked_up_without_touching_the_role() {
        let role = test_role(OverrideField { a: 0, d: 0 }, Some(RoleClass::Free));

        let mut class_defaults = HashMap::new();
        class_defaults.insert(
            RoleClass::Free,
            ClassDefault {
                permissions: OverrideField {
                    a: ChannelPermission::ViewChannel as i64,
                    d: 0,
                },
                channel_overrides: HashMap::new(),
                max_message_length: Some(1000),
            },
        );
        let server_before = test_server(class_defaults.clone());
        let resolved_before = resolve_role_base_override(&role, &server_before);
        assert_eq!(resolved_before.allow, ChannelPermission::ViewChannel as u64);

        // Simulate the server owner editing the class default - same role doc,
        // different server-level config.
        class_defaults.insert(
            RoleClass::Free,
            ClassDefault {
                permissions: OverrideField {
                    a: (ChannelPermission::ViewChannel as u64
                        | ChannelPermission::SendMessage as u64) as i64,
                    d: 0,
                },
                channel_overrides: HashMap::new(),
                max_message_length: Some(2000),
            },
        );
        let server_after = test_server(class_defaults);
        let resolved_after = resolve_role_base_override(&role, &server_after);
        assert_eq!(
            resolved_after.allow,
            ChannelPermission::ViewChannel as u64 | ChannelPermission::SendMessage as u64
        );
    }

    #[test]
    fn channel_override_falls_back_to_class_channel_template() {
        let role = test_role(OverrideField { a: 0, d: 0 }, Some(RoleClass::Member));

        let mut channel_overrides = HashMap::new();
        channel_overrides.insert(
            "channel-1".to_string(),
            OverrideField {
                a: ChannelPermission::SendMessage as i64,
                d: 0,
            },
        );

        let mut class_defaults = HashMap::new();
        class_defaults.insert(
            RoleClass::Member,
            ClassDefault {
                permissions: Default::default(),
                channel_overrides,
                max_message_length: None,
            },
        );
        let server = test_server(class_defaults);

        let resolved = resolve_role_channel_override(&role, &server, "channel-1", None);
        assert_eq!(resolved.allow, ChannelPermission::SendMessage as u64);

        // A channel with no template entry at all stays a no-op, same as today.
        let resolved_other_channel =
            resolve_role_channel_override(&role, &server, "channel-2", None);
        assert_eq!(resolved_other_channel.allow, 0);
        assert_eq!(resolved_other_channel.deny, 0);
    }

    #[test]
    fn explicit_channel_override_wins_over_class_channel_template() {
        let role = test_role(OverrideField { a: 0, d: 0 }, Some(RoleClass::Member));

        let mut channel_overrides = HashMap::new();
        channel_overrides.insert(
            "channel-1".to_string(),
            OverrideField {
                a: ChannelPermission::SendMessage as i64,
                d: 0,
            },
        );

        let mut class_defaults = HashMap::new();
        class_defaults.insert(
            RoleClass::Member,
            ClassDefault {
                permissions: Default::default(),
                channel_overrides,
                max_message_length: None,
            },
        );
        let server = test_server(class_defaults);

        let explicit = Some(revolt_permissions::Override {
            allow: 0,
            deny: ChannelPermission::SendMessage as u64,
        });

        let resolved = resolve_role_channel_override(&role, &server, "channel-1", explicit);
        assert_eq!(resolved.allow & ChannelPermission::SendMessage as u64, 0);
        assert_eq!(
            resolved.deny & ChannelPermission::SendMessage as u64,
            ChannelPermission::SendMessage as u64
        );
    }
}
