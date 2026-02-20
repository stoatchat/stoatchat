use std::collections::{HashMap, HashSet};

use futures::future::join_all;
use revolt_database::{
    events::client::{EventV1, ReadyPayloadFields},
    util::permissions::DatabasePermissionQuery,
    voice::get_channel_voice_state,
    Channel, Database, Member, MemberCompositeKey, RelationshipStatus,
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_presence::filter_online;
use revolt_result::Result;
use ulid::Ulid;

use super::state::{Cache, State};

/// Cache Manager
impl Cache {
    /// Check whether the current user can view a channel
    pub async fn can_view_channel(&self, db: &Database, channel: &Channel) -> bool {
        #[allow(deprecated)]
        match channel {
            Channel::TextChannel { server, .. } => {
                let user = self
                    .users
                    .get(&self.user_id)
                    .expect("self user missing in cache");
                let member = self.members.get(server);
                let server_obj = self.servers.get(server);

                let mut query = DatabasePermissionQuery::new(db, user).channel(channel);

                if let Some(m) = member {
                    query = query.member(m);
                }
                if let Some(s) = server_obj {
                    query = query.server(s);
                }

                calculate_channel_permissions(&mut query)
                    .await
                    .has_channel_permission(ChannelPermission::ViewChannel)
            }
            _ => true,
        }
    }

    /// Filter channels to only those the user can access
    pub async fn filter_accessible_channels(
        &self,
        db: &Database,
        channels: Vec<Channel>,
    ) -> Vec<Channel> {
        let mut viewable = Vec::with_capacity(channels.len());
        for channel in channels {
            if self.can_view_channel(db, &channel).await {
                viewable.push(channel);
            }
        }
        viewable
    }

    /// Check if we can subscribe to another user's events
    pub fn can_subscribe_to_user(&self, user_id: &str) -> bool {
        let Some(self_user) = self.users.get(&self.user_id) else {
            return false;
        };

        if matches!(
            self_user.relationship_with(user_id),
            RelationshipStatus::Friend
                | RelationshipStatus::Incoming
                | RelationshipStatus::Outgoing
                | RelationshipStatus::User
        ) {
            return true;
        }

        for channel in self.channels.values() {
            let recipients = match channel {
                Channel::DirectMessage { recipients, .. } | Channel::Group { recipients, .. } => recipients,
                _ => continue,
            };

            if recipients.iter().any(|r| r.as_str() == user_id) {
                return true;
            }
        }

        false
    }
}

/// State Manager
impl State {
    /// Generate Ready payload (initial sync)
    pub async fn generate_ready_payload(
        &mut self,
        db: &Database,
        fields: &ReadyPayloadFields,
    ) -> Result<EventV1> {
        let self_user = self.clone_user();
        self.cache.is_bot = self_user.bot.is_some();

        let policy_changes = if self_user.bot.is_some() || !fields.policy_changes {
            None
        } else {
            Some(
                db.fetch_policy_changes()
                    .await?
                    .into_iter()
                    .filter(|p| p.created_time > self_user.last_acknowledged_policy_change)
                    .map(Into::into)
                    .collect(),
            )
        };

        let mut known_user_ids: HashSet<String> = self_user
            .relations
            .as_ref()
            .map(|rels| rels.iter().map(|r| r.id.clone()).collect())
            .unwrap_or_default();

        let mut members: Vec<Member> = db.fetch_all_memberships(&self_user.id).await?;

        let server_ids: Vec<String> = members.iter().map(|m| m.id.server.clone()).collect();
        let servers = db.fetch_servers(&server_ids).await?;

        self.cache.servers = servers.iter().map(|s| (s.id.clone(), s.clone())).collect();

        let mut channel_ids = Vec::new();
        for server in &servers {
            channel_ids.extend_from_slice(&server.channels);
        }

        let mut channels = db.find_direct_messages(&self_user.id).await?;
        channels.extend(db.fetch_channels(&channel_ids).await?);

        let channels = self.cache.filter_accessible_channels(db, channels).await;

        for ch in &channels {
            if let Channel::DirectMessage { recipients, .. } | Channel::Group { recipients, .. } = ch {
                known_user_ids.extend(recipients.iter().cloned());
            }
        }

        let voice_states = if fields.voice_states {
            let mut server_to_members: HashMap<String, HashSet<String>> = HashMap::new();
            let mut states = Vec::new();

            for ch in channels.iter().filter(|c| matches!(
                c,
                Channel::DirectMessage { .. } | Channel::Group { .. } | Channel::TextChannel { voice: Some(_), .. }
            )) {
                let Ok(Some(vs)) = get_channel_voice_state(ch).await else { continue; };

                if let Some(srv_id) = ch.server() {
                    let entry = server_to_members.entry(srv_id.to_string()).or_default();
                    for p in &vs.participants {
                        known_user_ids.insert(p.id.clone());
                        entry.insert(p.id.clone());
                    }
                } else {
                    for p in &vs.participants {
                        known_user_ids.insert(p.id.clone());
                    }
                }
                states.push(vs);
            }

            for (server_id, member_set) in server_to_members {
                let member_list: Vec<String> = member_set.into_iter().collect();
                let extra_members = db.fetch_members(&server_id, &member_list).await?;
                members.extend(extra_members);
            }

            Some(states)
        } else {
            None
        };

        let online_ids = filter_online(&known_user_ids.iter().cloned().collect::<Vec<_>>()).await;

        // Cache users as v0::User with optimized lookup
        let mut user_ids_to_fetch: Vec<String> = known_user_ids
            .iter()
            .filter(|&uid| uid != &self_user.id)
            .cloned()
            .collect();
        
        // Remove already cached users to avoid unnecessary DB queries
        user_ids_to_fetch.retain(|uid| !self.cache.users.contains_key(uid));
        
        let other_users = if !user_ids_to_fetch.is_empty() {
            db.fetch_users(&user_ids_to_fetch).await?
        } else {
            Vec::new()
        };

        // Update cache with newly fetched users
        for user in &other_users {
            self.cache.users.insert(user.id.clone(), user.clone());
        }
        self.cache.users.insert(self_user.id.clone(), self_user.clone());

        self.cache.members = members.iter().map(|m| (m.id.server.clone(), m.clone())).collect();

        let emojis = if fields.emojis {
            let parent_ids = servers.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
            Some(
                db.fetch_emoji_by_parent_ids(&parent_ids)
                    .await?
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            )
        } else {
            None
        };

        let user_settings = if !fields.user_settings.is_empty() {
            Some(db.fetch_user_settings(&self_user.id, &fields.user_settings).await?)
        } else {
            None
        };

        let channel_unreads = if fields.channel_unreads {
            Some(
                db.fetch_unreads(&self_user.id)
                    .await?
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            )
        } else {
            None
        };

        self.cache.channels = channels
            .iter()
            .map(|ch| (ch.id().to_string(), ch.clone()))
            .collect();

        // Convert to client-visible form, including cached users
        let mut all_users: Vec<revolt_database::User> = other_users;
        
        // Add cached users that are in known_user_ids but not in other_users
        for uid in &known_user_ids {
            if uid != &self_user.id && !all_users.iter().any(|u| &u.id == uid) {
                if let Some(cached_user) = self.cache.users.get(uid) {
                    all_users.push(cached_user.clone());
                }
            }
        }
        
        let mut visible_users: Vec<v0::User> = join_all(all_users.into_iter().map(|u| async {
            let is_online = online_ids.contains(&u.id);
            u.into_known(&self_user, is_online).await
        }))
        .await;

        visible_users.push(self_user.into_self(true).await);

        self.reset_state().await;
        self.insert_subscription(self.private_topic.clone()).await;

        for u in &visible_users {
            self.insert_subscription(u.id.clone()).await;
        }
        for srv in &servers {
            self.insert_subscription(srv.id.clone()).await;
            if self.cache.is_bot {
                self.insert_subscription(format!("{}u", srv.id)).await;
            }
        }
        for ch in &channels {
            self.insert_subscription(ch.id().to_string()).await;
        }

        Ok(EventV1::Ready {
            users: fields.users.then_some(visible_users),
            servers: fields.servers.then_some(servers.into_iter().map(Into::into).collect()),
            channels: fields.channels.then_some(channels.into_iter().map(Into::into).collect()),
            members: fields.members.then_some(members.into_iter().map(Into::into).collect()),
            voice_states,
            emojis,
            user_settings,
            channel_unreads,
            policy_changes,
        })
    }

    pub async fn recalculate_server(&mut self, db: &Database, server_id: &str, event: &mut EventV1) {
        let Some(server) = self.cache.servers.get(server_id) else { return; };

        let mut cached_channel_ids = HashSet::new();
        let mut to_add = Vec::new();
        let mut to_remove = Vec::new();

        let server_id_str = server_id.to_string();
        for (ch_id, ch) in &self.cache.channels {
            if ch.server() == Some(&server_id_str) {
                cached_channel_ids.insert(ch_id.clone());

                if self.cache.can_view_channel(db, ch).await {
                    to_add.push(ch_id.clone());
                } else {
                    to_remove.push(ch_id.clone());
                }
            }
        }

        let known_channel_ids = server.channels.iter().cloned().collect::<HashSet<_>>();

        let mut bulk = Vec::new();

        for id in to_add {
            self.insert_subscription(id).await;
        }

        for id in to_remove {
            self.remove_subscription(&id).await;
            self.cache.channels.remove(&id);
            bulk.push(EventV1::ChannelDelete { id });
        }

        let missing = known_channel_ids
            .difference(&cached_channel_ids)
            .cloned()
            .collect::<Vec<_>>();

        if !missing.is_empty() {
            if let Ok(fetched) = db.fetch_channels(&missing).await {
                let viewable = self.cache.filter_accessible_channels(db, fetched).await;
                for ch in viewable {
                    let ch_id = ch.id().to_string();
                    self.cache.channels.insert(ch_id.clone(), ch.clone());
                    self.insert_subscription(ch_id).await;
                    bulk.push(EventV1::ChannelCreate(ch.into()));
                }
            }
        }

        if !bulk.is_empty() {
            let mut new_bulk = EventV1::Bulk { v: bulk };
            std::mem::swap(&mut new_bulk, event);
            if let EventV1::Bulk { v } = event {
                v.push(new_bulk);
            }
        }
    }

    pub async fn broadcast_presence_change(&self, target: bool) {
        // Check if user is not invisible before broadcasting presence change
        if let Some(user) = self.cache.users.get(&self.cache.user_id) {
            if let Some(status) = &user.status {
                if status.presence == Some(revolt_database::Presence::Invisible) {
                    return; // Don't broadcast if user is invisible
                }
            }
            
            // Create UserUpdate event for presence change
            let event = EventV1::UserUpdate {
                id: self.cache.user_id.clone(),
                data: revolt_models::v0::PartialUser {
                    online: Some(target),
                    ..Default::default()
                },
                clear: vec![],
                event_id: Some(Ulid::new().to_string()),
            };

            // Broadcast to all servers the user is a member of
            for server_id in self.cache.servers.keys() {
                let event_clone = event.clone();
                event_clone.p(server_id.clone()).await;
            }

            // Also broadcast to user's own session
            event.p(self.cache.user_id.clone()).await;
        }
    }

    /// Handle incoming v1 event – critical for live updates (avatars, etc.)
    pub async fn handle_incoming_event_v1(&mut self, db: &Database, event: &mut EventV1) -> bool {
        let mut recalc_server = None;
        let mut queue_sub_add = None;
        let mut queue_sub_remove = None;

        match event {
            EventV1::ChannelCreate(channel) => {
                let id = channel.id().to_string();
                self.insert_subscription(id.clone()).await;
                self.cache.channels.insert(id, channel.clone().into());
            }
            EventV1::ChannelUpdate { id, data, clear, .. } => {
                let could_view = if let Some(c) = self.cache.channels.get(id) {
                    self.cache.can_view_channel(db, c).await
                } else {
                    false
                };

                if let Some(ch) = self.cache.channels.get_mut(id) {
                    for field in clear {
                        ch.remove_field(&field.clone().into());
                    }
                    ch.apply_options(data.clone().into());
                }

                if !self.cache.channels.contains_key(id) {
                    if let Ok(ch) = db.fetch_channel(id).await {
                        self.cache.channels.insert(id.clone(), ch);
                    }
                }

                if let Some(ch) = self.cache.channels.get(id) {
                    let now_viewable = self.cache.can_view_channel(db, ch).await;
                    if could_view != now_viewable {
                        if now_viewable {
                            queue_sub_add = Some(id.clone());
                            *event = EventV1::ChannelCreate(ch.clone().into());
                        } else {
                            queue_sub_remove = Some(id.clone());
                            *event = EventV1::ChannelDelete { id: id.clone() };
                        }
                    }
                }
            }
            EventV1::ChannelDelete { id } => {
                self.remove_subscription(id).await;
                self.cache.channels.remove(id);
            }
            EventV1::ChannelGroupJoin { user, .. } => {
                self.insert_subscription(user.clone()).await;
            }
            EventV1::ChannelGroupLeave { id, user, .. } => {
                if user == &self.cache.user_id {
                    self.remove_subscription(id).await;
                } else if !self.cache.can_subscribe_to_user(user) {
                    self.remove_subscription(user).await;
                }
            }
            EventV1::ServerCreate { id, server, channels, .. } => {
                self.insert_subscription(id.clone()).await;
                if self.cache.is_bot {
                    self.insert_subscription(format!("{}u", id)).await;
                }

                self.cache.servers.insert(id.clone(), server.clone().into());
                let member = Member {
                    id: MemberCompositeKey {
                        server: server.id.clone(),
                        user: self.cache.user_id.clone(),
                    },
                    ..Default::default()
                };
                self.cache.members.insert(id.clone(), member);

                for ch in channels {
                    self.cache.channels.insert(ch.id().to_string(), ch.clone().into());
                }

                recalc_server = Some(id.clone());
            }
            EventV1::ServerUpdate { id, data, clear, .. } => {
                if let Some(srv) = self.cache.servers.get_mut(id) {
                    for field in clear {
                        srv.remove_field(&field.clone().into());
                    }
                    srv.apply_options(data.clone().into());
                }
                if data.default_permissions.is_some() {
                    recalc_server = Some(id.clone());
                }
            }
            EventV1::ServerMemberLeave { id, user, .. } => {
                if user == &self.cache.user_id {
                    self.remove_subscription(id).await;
                    if let Some(srv) = self.cache.servers.remove(id) {
                        for ch_id in &srv.channels {
                            self.remove_subscription(ch_id).await;
                            self.cache.channels.remove(ch_id);
                        }
                    }
                    self.cache.members.remove(id);
                }
            }
            EventV1::ServerDelete { id } => {
                self.remove_subscription(id).await;
                if let Some(srv) = self.cache.servers.remove(id) {
                    for ch_id in &srv.channels {
                        self.remove_subscription(ch_id).await;
                        self.cache.channels.remove(ch_id);
                    }
                }
                self.cache.members.remove(id);
            }
            EventV1::ServerMemberUpdate { id, data, clear } => {
                if id.user == self.cache.user_id {
                    let clear_clone = clear.clone();
                    if let Some(mem) = self.cache.members.get_mut(&id.server) {
                        for field in clear {
                            mem.remove_field(&field.clone().into());
                        }
                        mem.apply_options(data.clone().into());
                    }
                    if data.roles.is_some() || clear_clone.contains(&v0::FieldsMember::Roles) {
                        recalc_server = Some(id.server.clone());
                    }
                }
            }
            EventV1::ServerRoleUpdate { id, role_id, data, clear, .. } => {
                if let Some(srv) = self.cache.servers.get_mut(id) {
                    if let Some(role) = srv.roles.get_mut(role_id) {
                        for field in clear {
                            role.remove_field(&field.clone().into());
                        }
                        role.apply_options(data.clone().into());
                    }
                }
                if data.rank.is_some() || data.permissions.is_some() {
                    if let Some(mem) = self.cache.members.get(id) {
                        if mem.roles.contains(role_id) {
                            recalc_server = Some(id.clone());
                        }
                    }
                }
            }
            EventV1::ServerRoleDelete { id, role_id } => {
                if let Some(srv) = self.cache.servers.get_mut(id) {
                    srv.roles.remove(role_id);
                }
                if let Some(mem) = self.cache.members.get(id) {
                    if mem.roles.contains(role_id) {
                        recalc_server = Some(id.clone());
                    }
                }
            }
            EventV1::UserUpdate { id, data, clear, event_id } => {
                if let Some(eid) = event_id {
                    if self.cache.seen_events.contains(eid) {
                        return false;
                    }
                    self.cache.seen_events.put(eid.clone(), ());
                }

                // Early return for non-avatar updates to reduce processing overhead
                let has_avatar_update = data.avatar.is_some() || clear.contains(&revolt_models::v0::FieldsUser::Avatar);
                let has_essential_update = data.display_name.is_some() 
                    || data.status.is_some()
                    || clear.iter().any(|f| matches!(f, revolt_models::v0::FieldsUser::DisplayName | revolt_models::v0::FieldsUser::StatusText | revolt_models::v0::FieldsUser::StatusPresence));

                if !has_avatar_update && !has_essential_update {
                    *event_id = None;
                    return true; // Skip processing for non-essential updates
                }

                // Update existing cached user with avatar and other field changes
                if let Some(cached_user) = self.cache.users.get_mut(id) {
                    // Apply avatar updates
                    if let Some(avatar) = &data.avatar {
                        cached_user.avatar = Some(avatar.clone().into());
                    }
                    
                    // Apply display name updates
                    if let Some(display_name) = &data.display_name {
                        cached_user.display_name = Some(display_name.clone());
                    }
                    
                    // Apply status updates (convert from v0::UserStatus to database::UserStatus)
                    if let Some(status) = &data.status {
                        if cached_user.status.is_none() {
                            cached_user.status = Some(revolt_database::UserStatus::default());
                        }
                        
                        if let Some(ref mut cached_status) = cached_user.status {
                            if let Some(ref text) = status.text {
                                cached_status.text = Some(text.clone());
                            }
                            
                            if let Some(ref presence) = status.presence {
                                cached_status.presence = Some(match presence {
                                    revolt_models::v0::Presence::Online => revolt_database::Presence::Online,
                                    revolt_models::v0::Presence::Idle => revolt_database::Presence::Idle,
                                    revolt_models::v0::Presence::Focus => revolt_database::Presence::Focus,
                                    revolt_models::v0::Presence::Busy => revolt_database::Presence::Busy,
                                    revolt_models::v0::Presence::Invisible => revolt_database::Presence::Invisible,
                                });
                            }
                        }
                    }
                    
                    // Handle field removals
                    for field in clear {
                        match field {
                            revolt_models::v0::FieldsUser::Avatar => {
                                cached_user.avatar = None;
                            }
                            revolt_models::v0::FieldsUser::StatusText => {
                                if let Some(ref mut status) = cached_user.status {
                                    status.text = None;
                                }
                            }
                            revolt_models::v0::FieldsUser::StatusPresence => {
                                if let Some(ref mut status) = cached_user.status {
                                    status.presence = None;
                                }
                            }
                            revolt_models::v0::FieldsUser::DisplayName => {
                                cached_user.display_name = None;
                            }
                            revolt_models::v0::FieldsUser::ProfileContent => {
                                if let Some(ref mut profile) = cached_user.profile {
                                    profile.content = None;
                                }
                            }
                            revolt_models::v0::FieldsUser::ProfileBackground => {
                                if let Some(ref mut profile) = cached_user.profile {
                                    profile.background = None;
                                }
                            }
                            revolt_models::v0::FieldsUser::Internal => {
                                // Skip internal fields
                            }
                        }
                    }
                }
                // Fetch from DB if missing from cache and this is an essential update
                else if has_essential_update {
                    if let Ok(mut db_user) = db.fetch_user(id).await {
                        // Apply the updates to the freshly fetched user
                        if let Some(avatar) = &data.avatar {
                            db_user.avatar = Some(avatar.clone().into());
                        }
                        
                        if let Some(display_name) = &data.display_name {
                            db_user.display_name = Some(display_name.clone());
                        }
                        
                        if let Some(status) = &data.status {
                            if db_user.status.is_none() {
                                db_user.status = Some(revolt_database::UserStatus::default());
                            }
                            
                            if let Some(ref mut db_status) = db_user.status {
                                if let Some(ref text) = status.text {
                                    db_status.text = Some(text.clone());
                                }
                                
                                if let Some(ref presence) = status.presence {
                                    db_status.presence = Some(match presence {
                                        revolt_models::v0::Presence::Online => revolt_database::Presence::Online,
                                        revolt_models::v0::Presence::Idle => revolt_database::Presence::Idle,
                                        revolt_models::v0::Presence::Focus => revolt_database::Presence::Focus,
                                        revolt_models::v0::Presence::Busy => revolt_database::Presence::Busy,
                                        revolt_models::v0::Presence::Invisible => revolt_database::Presence::Invisible,
                                    });
                                }
                            }
                        }

                        self.cache.users.insert(id.clone(), db_user);
                    }
                }

                *event_id = None;
            }
            EventV1::UserRelationship { id, user, .. } => {
                self.cache.users.insert(id.clone(), user.clone().into());

                if self.cache.can_subscribe_to_user(id) {
                    self.insert_subscription(id.clone()).await;
                } else {
                    self.remove_subscription(id).await;
                }
            }
            EventV1::Message(message) => {
                if let Some(user) = &mut message.user {
                    if let Some(self_user) = self.cache.users.get(&self.cache.user_id) {
                        user.relationship = self_user.relationship_with(&message.author).into();
                    }
                }
            }
            _ => {}
        }

        if let Some(sid) = recalc_server {
            self.recalculate_server(db, &sid, event).await;
        }

        if let Some(id) = queue_sub_add {
            self.insert_subscription(id).await;
        }
        if let Some(id) = queue_sub_remove {
            self.remove_subscription(&id).await;
        }

        true
    }
}
