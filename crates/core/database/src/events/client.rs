use authifier::AuthifierEvent;
use revolt_result::Error;
use serde::{Deserialize, Serialize};

use revolt_models::v0::{
    AppendMessage, Channel, ChannelUnread, ChannelVoiceState, Emoji, FieldsChannel, FieldsMember, FieldsMessage, FieldsRole, FieldsServer, FieldsUser, FieldsWebhook, Member, MemberCompositeKey, Message, PartialChannel, PartialMember, PartialMessage, PartialRole, PartialServer, PartialUser, PartialUserVoiceState, PartialWebhook, PolicyChange, RemovalIntention, Report, Server, User, UserSettings, UserVoiceState, Webhook
};

use crate::Database;

/// Ping Packet
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Ping {
    Binary(Vec<u8>),
    Number(usize),
}

/// Fields provided in Ready payload
#[derive(PartialEq, Debug, Clone, Deserialize)]
pub struct ReadyPayloadFields {
    pub users: bool,
    pub servers: bool,
    pub channels: bool,
    pub members: bool,
    pub emojis: bool,
    pub voice_states: bool,
    pub user_settings: Vec<String>,
    pub channel_unreads: bool,
    pub policy_changes: bool,
}

impl Default for ReadyPayloadFields {
    fn default() -> Self {
        Self {
            users: true,
            servers: true,
            channels: true,
            members: true,
            emojis: true,
            voice_states: true,
            user_settings: Vec::new(),
            channel_unreads: false,
            policy_changes: true,
        }
    }
}

/// Protocol Events
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum EventV1 {
    /// Multiple events
    Bulk { v: Vec<EventV1> },
    /// Error event
    Error { data: Error },

    /// Successfully authenticated
    Authenticated,
    /// Logged out
    Logout,
    /// Basic data to cache
    Ready {
        #[serde(skip_serializing_if = "Option::is_none")]
        users: Option<Vec<User>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        servers: Option<Vec<Server>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        channels: Option<Vec<Channel>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        members: Option<Vec<Member>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        emojis: Option<Vec<Emoji>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        voice_states: Option<Vec<ChannelVoiceState>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        user_settings: Option<UserSettings>,
        #[serde(skip_serializing_if = "Option::is_none")]
        channel_unreads: Option<Vec<ChannelUnread>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        policy_changes: Option<Vec<PolicyChange>>,
    },

    /// Ping response
    Pong { data: Ping },
    /// New message
    Message(Message),

    /// Update existing message
    MessageUpdate {
        id: String,
        channel: String,
        data: PartialMessage,
        #[serde(default)]
        clear: Vec<FieldsMessage>,
    },

    /// Append information to existing message
    MessageAppend {
        id: String,
        channel: String,
        append: AppendMessage,
    },

    /// Delete message
    MessageDelete { id: String, channel: String },

    /// New reaction to a message
    MessageReact {
        id: String,
        channel_id: String,
        user_id: String,
        emoji_id: String,
    },

    /// Remove user's reaction from message
    MessageUnreact {
        id: String,
        channel_id: String,
        user_id: String,
        emoji_id: String,
    },

    /// Remove a reaction from message
    MessageRemoveReaction {
        id: String,
        channel_id: String,
        emoji_id: String,
    },

    /// Bulk delete messages
    BulkMessageDelete { channel: String, ids: Vec<String> },

    /// New server
    ServerCreate {
        id: String,
        server: Server,
        channels: Vec<Channel>,
        emojis: Vec<Emoji>,
        voice_states: Vec<ChannelVoiceState>
    },

    /// Update existing server
    ServerUpdate {
        id: String,
        data: PartialServer,
        #[serde(default)]
        clear: Vec<FieldsServer>,
    },

    /// Delete server
    ServerDelete { id: String },

    /// Update existing server member
    ServerMemberUpdate {
        id: MemberCompositeKey,
        data: PartialMember,
        #[serde(default)]
        clear: Vec<FieldsMember>,
    },

    /// User joins server
    ServerMemberJoin {
        id: String,
        // Deprecated: use member.id.user
        #[deprecated = "Use member.id.user instead"]
        user: String,
        member: Member,
    },

    /// User left server
    ServerMemberLeave {
        id: String,
        user: String,
        reason: RemovalIntention,
    },

    /// Server role created or updated
    ServerRoleUpdate {
        id: String,
        role_id: String,
        data: PartialRole,
        #[serde(default)]
        clear: Vec<FieldsRole>,
    },

    /// Server role deleted
    ServerRoleDelete { id: String, role_id: String },

    /// Server roles ranks updated
    ServerRoleRanksUpdate { id: String, ranks: Vec<String> },

    /// Update existing user
    UserUpdate {
        id: String,
        data: PartialUser,
        #[serde(default)]
        clear: Vec<FieldsUser>,
        event_id: Option<String>,
    },

    /// Relationship with another user changed
    UserRelationship { id: String, user: User },
    /// Settings updated remotely
    UserSettingsUpdate { id: String, update: UserSettings },

    /// User has been platform banned or deleted their account
    ///
    /// Clients should remove the following associated data:
    /// - Messages
    /// - DM Channels
    /// - Relationships
    /// - Server Memberships
    ///
    /// User flags are specified to explain why a wipe is occurring though not all reasons will necessarily ever appear.
    UserPlatformWipe { user_id: String, flags: i32 },
    /// New emoji
    EmojiCreate(Emoji),

    /// Delete emoji
    EmojiDelete { id: String },

    /// New report
    ReportCreate(Report),
    /// New channel
    ChannelCreate(Channel),

    /// Update existing channel
    ChannelUpdate {
        id: String,
        data: PartialChannel,
        #[serde(default)]
        clear: Vec<FieldsChannel>,
    },

    /// Delete channel
    ChannelDelete { id: String },

    /// User joins a group
    ChannelGroupJoin { id: String, user: String },

    /// User leaves a group
    ChannelGroupLeave { id: String, user: String },

    /// User started typing in a channel
    ChannelStartTyping { id: String, user: String },

    /// User stopped typing in a channel
    ChannelStopTyping { id: String, user: String },

    /// User acknowledged message in channel
    ChannelAck {
        id: String,
        user: String,
        message_id: String,
    },

    /// New webhook
    WebhookCreate(Webhook),

    /// Update existing webhook
    WebhookUpdate {
        id: String,
        data: PartialWebhook,
        remove: Vec<FieldsWebhook>,
    },

    /// Delete webhook
    WebhookDelete { id: String },

    /// Auth events
    Auth(AuthifierEvent),

    /// Voice events
    VoiceChannelJoin {
        id: String,
        state: UserVoiceState,
    },
    VoiceChannelLeave {
        id: String,
        user: String,
    },
    VoiceChannelMove {
        user: String,
        from: String,
        to: String,
        state: UserVoiceState
    },
    UserVoiceStateUpdate {
        id: String,
        channel_id: String,
        data: PartialUserVoiceState,
    },
    UserMoveVoiceChannel {
        node: String,
        from: String,
        to: String,
        token: String,
    }
}

impl EventV1 {
    /// Publish helper wrapper
    pub async fn p(self, channel: String) {
        #[cfg(not(debug_assertions))]
        redis_kiss::p(channel, self).await;

        #[cfg(debug_assertions)]
        info!("Publishing event to {channel}: {self:?}");

        #[cfg(debug_assertions)]
        redis_kiss::publish(channel, self).await.unwrap();
    }

    /// Publish user event
    pub async fn p_user(self, id: String, db: &Database) {
        self.clone().p(id.clone()).await;

        // TODO: this should be captured by member list in the future and not immediately fanned out to users
        if let Ok(members) = db.fetch_all_memberships(&id).await {
            for member in members {
                self.clone().server(member.id.server).await;
            }
        }
    }

    /// Publish private event
    pub async fn private(self, id: String) {
        self.p(format!("{id}!")).await;
    }

    /// Publish server member event
    pub async fn server(self, id: String) {
        self.p(format!("{id}u")).await;
    }

    /// Publish internal global event
    pub async fn global(self) {
        self.p("global".to_string()).await;
    }
}
