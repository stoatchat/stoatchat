use std::collections::{HashMap, HashSet};

use revolt_models::v0::{self, DataCreateServerChannel};
use revolt_permissions::{OverrideField, DEFAULT_PERMISSION_SERVER};
use revolt_result::Result;
use ulid::Ulid;

use crate::{events::client::EventV1, Channel, Database, File, User};

auto_derived_partial!(
    /// Server
    pub struct Server {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,
        /// User id of the owner
        pub owner: String,

        /// Name of the server
        pub name: String,
        /// Description for the server
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,

        /// Channels within this server
        // TODO: investigate if this is redundant and can be removed
        pub channels: Vec<String>,
        /// Categories for this server
        #[serde(skip_serializing_if = "Option::is_none")]
        pub categories: Option<Vec<Category>>,
        /// Configuration for sending system event messages
        #[serde(skip_serializing_if = "Option::is_none")]
        pub system_messages: Option<SystemMessageChannels>,

        /// Roles for this server
        #[serde(
            default = "HashMap::<String, Role>::new",
            skip_serializing_if = "HashMap::<String, Role>::is_empty"
        )]
        pub roles: HashMap<String, Role>,
        /// Default set of server and channel permissions
        pub default_permissions: i64,

        /// Icon attachment
        #[serde(skip_serializing_if = "Option::is_none")]
        pub icon: Option<File>,
        /// Banner attachment
        #[serde(skip_serializing_if = "Option::is_none")]
        pub banner: Option<File>,

        /// Bitfield of server flags
        #[serde(skip_serializing_if = "Option::is_none")]
        pub flags: Option<i32>,

        /// Whether this server is flagged as not safe for work
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub nsfw: bool,
        /// Whether to enable analytics
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub analytics: bool,
        /// Whether this server should be publicly discoverable
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub discoverable: bool,
    },
    "PartialServer"
);

auto_derived_partial!(
    /// Role
    pub struct Role {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,
        /// Role name
        pub name: String,
        /// Permissions available to this role
        pub permissions: OverrideField,
        /// Colour used for this role
        ///
        /// This can be any valid CSS colour
        #[serde(skip_serializing_if = "Option::is_none")]
        pub colour: Option<String>,
        /// Whether this role should be shown separately on the member sidebar
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub hoist: bool,
        /// Ranking of this role
        #[serde(default)]
        pub rank: i64,
    },
    "PartialRole"
);

auto_derived!(
    /// Channel category
    pub struct Category {
        /// Unique ID for this category
        pub id: String,
        /// Title for this category
        pub title: String,
        /// Channels in this category
        pub channels: Vec<String>,
    }

    /// System message channel assignments
    pub struct SystemMessageChannels {
        /// ID of channel to send user join messages in
        #[serde(skip_serializing_if = "Option::is_none")]
        pub user_joined: Option<String>,
        /// ID of channel to send user left messages in
        #[serde(skip_serializing_if = "Option::is_none")]
        pub user_left: Option<String>,
        /// ID of channel to send user kicked messages in
        #[serde(skip_serializing_if = "Option::is_none")]
        pub user_kicked: Option<String>,
        /// ID of channel to send user banned messages in
        #[serde(skip_serializing_if = "Option::is_none")]
        pub user_banned: Option<String>,
    }

    /// Optional fields on server object
    pub enum FieldsServer {
        Description,
        Categories,
        SystemMessages,
        Icon,
        Banner,
    }

    /// Optional fields on server object
    pub enum FieldsRole {
        Colour,
    }
);

#[allow(clippy::disallowed_methods)]
impl Server {
    /// Create a server
    pub async fn create(
        db: &Database,
        data: v0::DataCreateServer,
        owner: &User,
        create_default_channels: bool,
    ) -> Result<(Server, Vec<Channel>)> {
        let mut server = Server {
            id: ulid::Ulid::new().to_string(),
            owner: owner.id.to_string(),
            name: data.name,
            description: data.description,
            channels: vec![],
            nsfw: data.nsfw.unwrap_or(false),
            default_permissions: *DEFAULT_PERMISSION_SERVER as i64,

            analytics: false,
            banner: None,
            categories: None,
            discoverable: false,
            flags: None,
            icon: None,
            roles: HashMap::new(),
            system_messages: None,
        };

        let channels: Vec<Channel> = if create_default_channels {
            vec![
                Channel::create_server_channel(
                    db,
                    &mut server,
                    DataCreateServerChannel {
                        channel_type: v0::LegacyServerChannelType::Text,
                        name: "General".to_string(),
                        ..Default::default()
                    },
                    false,
                )
                .await?,
            ]
        } else {
            vec![]
        };

        server.channels = channels.iter().map(|c| c.id().to_string()).collect();
        db.insert_server(&server).await?;
        Ok((server, channels))
    }

    /// Update server data
    pub async fn update(
        &mut self,
        db: &Database,
        partial: PartialServer,
        remove: Vec<FieldsServer>,
    ) -> Result<()> {
        for field in &remove {
            self.remove_field(field);
        }

        self.apply_options(partial.clone());

        db.update_server(&self.id, &partial, remove.clone()).await?;

        EventV1::ServerUpdate {
            id: self.id.clone(),
            data: partial.into(),
            clear: remove.into_iter().map(|v| v.into()).collect(),
        }
        .p(self.id.clone())
        .await;

        Ok(())
    }

    /// Delete a server
    pub async fn delete(self, db: &Database) -> Result<()> {
        EventV1::ServerDelete {
            id: self.id.clone(),
        }
        .p(self.id.clone())
        .await;

        db.delete_server(&self.id).await
    }

    /// Remove a field from Server
    pub fn remove_field(&mut self, field: &FieldsServer) {
        match field {
            FieldsServer::Description => self.description = None,
            FieldsServer::Categories => self.categories = None,
            FieldsServer::SystemMessages => self.system_messages = None,
            FieldsServer::Icon => self.icon = None,
            FieldsServer::Banner => self.banner = None,
        }
    }

    /// Ordered roles list
    pub fn ordered_roles(&self) -> Vec<(String, Role)> {
        let mut ordered_roles = self.roles.clone().into_iter().collect::<Vec<_>>();
        ordered_roles.sort_by(|(_, role_a), (_, role_b)| role_a.rank.cmp(&role_b.rank));
        ordered_roles
    }

    /// Set role permission on a server
    pub async fn set_role_permission(
        &mut self,
        db: &Database,
        role_id: &str,
        permissions: OverrideField,
    ) -> Result<()> {
        if let Some(role) = self.roles.get_mut(role_id) {
            role.update(
                db,
                &self.id,
                PartialRole {
                    permissions: Some(permissions),
                    ..Default::default()
                },
                vec![],
            )
            .await?;

            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Reorders the server's roles rankings
    pub async fn set_role_ordering(&mut self, db: &Database, new_order: Vec<String>) -> Result<()> {
        // New order must always contain every role
        debug_assert_eq!(self.roles.len(), new_order.len());

        // Set the role's ranks to the positions in the vec
        for (rank, id) in new_order.iter().enumerate() {
            self.roles.get_mut(id).unwrap().rank = rank as i64;
        }

        db.update_server(
            &self.id,
            &PartialServer {
                roles: Some(self.roles.clone()),
                ..Default::default()
            },
            Vec::new(),
        )
        .await?;

        // Publish bulk update event
        EventV1::ServerRoleRanksUpdate {
            id: self.id.clone(),
            ranks: new_order,
        }
        .p(self.id.clone())
        .await;

        Ok(())
    }
}

impl Role {
    /// Into optional struct
    pub fn into_optional(self) -> PartialRole {
        PartialRole {
            id: Some(self.id),
            name: Some(self.name),
            permissions: Some(self.permissions),
            colour: self.colour,
            hoist: Some(self.hoist),
            rank: Some(self.rank),
        }
    }

    /// Create a role
    pub async fn create(db: &Database, server: &Server, name: String) -> Result<Self> {
        let role = Role {
            id: Ulid::new().to_string(),
            name,
            // Rank of the new role should be below the lowest role
            rank: server.roles.len() as i64,
            colour: None,
            hoist: false,
            permissions: Default::default(),
        };

        db.insert_role(&server.id, &role).await?;

        EventV1::ServerRoleUpdate {
            id: server.id.clone(),
            role_id: role.id.clone(),
            data: role.clone().into_optional().into(),
            clear: vec![],
        }
        .p(server.id.clone())
        .await;

        Ok(role)
    }

    /// Update server data
    pub async fn update(
        &mut self,
        db: &Database,
        server_id: &str,
        partial: PartialRole,
        remove: Vec<FieldsRole>,
    ) -> Result<()> {
        for field in &remove {
            self.remove_field(field);
        }

        self.apply_options(partial.clone());

        db.update_role(server_id, &self.id, &partial, remove.clone())
            .await?;

        EventV1::ServerRoleUpdate {
            id: server_id.to_string(),
            role_id: self.id.clone(),
            data: partial.into(),
            clear: remove.into_iter().map(Into::into).collect(),
        }
        .p(server_id.to_string())
        .await;

        Ok(())
    }

    /// Remove field from Role
    pub fn remove_field(&mut self, field: &FieldsRole) {
        match field {
            FieldsRole::Colour => self.colour = None,
        }
    }

    /// Delete a role
    pub async fn delete(self, db: &Database, server_id: &str) -> Result<()> {
        EventV1::ServerRoleDelete {
            id: server_id.to_string(),
            role_id: self.id.clone(),
        }
        .p(server_id.to_string())
        .await;

        db.delete_role(server_id, &self.id).await
    }
}

impl SystemMessageChannels {
    pub fn into_channel_ids(self) -> HashSet<String> {
        let mut ids = HashSet::new();

        if let Some(id) = self.user_joined {
            ids.insert(id);
        }

        if let Some(id) = self.user_left {
            ids.insert(id);
        }

        if let Some(id) = self.user_kicked {
            ids.insert(id);
        }

        if let Some(id) = self.user_banned {
            ids.insert(id);
        }

        ids
    }
}

#[cfg(test)]
mod tests {
    use revolt_permissions::{calculate_server_permissions, ChannelPermission};

    use crate::{fixture, util::permissions::DatabasePermissionQuery};

    #[async_std::test]
    async fn permissions() {
        database_test!(|db| async move {
            fixture!(db, "server_with_roles",
                owner user 0
                moderator user 1
                user user 2
                server server 4);

            let mut query = DatabasePermissionQuery::new(&db, &owner).server(&server);
            assert!(calculate_server_permissions(&mut query)
                .await
                .has_channel_permission(ChannelPermission::GrantAllSafe));

            let mut query = DatabasePermissionQuery::new(&db, &moderator).server(&server);
            assert!(calculate_server_permissions(&mut query)
                .await
                .has_channel_permission(ChannelPermission::BanMembers));

            let mut query = DatabasePermissionQuery::new(&db, &user).server(&server);
            assert!(!calculate_server_permissions(&mut query)
                .await
                .has_channel_permission(ChannelPermission::BanMembers));
        });
    }
}
