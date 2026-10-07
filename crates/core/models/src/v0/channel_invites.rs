use iso8601_timestamp::Timestamp;
use super::{Channel, File, Server, User};

auto_derived!(
    /// Invite
    #[serde(tag = "type")]
    pub enum Invite {
        /// Invite to a specific server channel
        Server {
            /// Invite code
            #[cfg_attr(feature = "serde", serde(rename = "_id"))]
            code: String,
            /// Id of the server this invite points to
            server: String,
            /// Id of user who created this invite
            creator: String,
            /// Id of the server channel this invite points to
            channel: String,
            /// Maximum number of times this invite can be used
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            max_uses: Option<u64>,
            /// Number of times this invite has been used
            uses: u64,
            /// Timestamp at which this invite expires
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            expires: Option<Timestamp>,
        },
        /// Invite to a group channel
        Group {
            /// Invite code
            #[cfg_attr(feature = "serde", serde(rename = "_id"))]
            code: String,
            /// Id of user who created this invite
            creator: String,
            /// Id of the group channel this invite points to
            channel: String,
            /// Maximum number of times this invite can be used
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            max_uses: Option<u64>,
            /// Number of times this invite has been used
            uses: u64,
            /// Timestamp at which this invite expires
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            expires: Option<Timestamp>,
        },
    }

    /// Create Invite
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct DataCreateInvite {
        /// Maximum number of times this invite can be used
        #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
        pub max_uses: Option<u64>,
        /// Number of seconds until this invite expires
        pub expires: Option<Timestamp>,
    }

    /// Public invite response
    #[allow(clippy::large_enum_variant)]
    #[serde(tag = "type")]
    pub enum InviteResponse {
        /// Server channel invite
        Server {
            /// Invite code
            code: String,
            /// Id of the server
            server_id: String,
            /// Name of the server
            server_name: String,
            /// Attachment for server icon
            #[serde(skip_serializing_if = "Option::is_none")]
            server_icon: Option<File>,
            /// Attachment for server banner
            #[serde(skip_serializing_if = "Option::is_none")]
            server_banner: Option<File>,
            /// Enum of server flags
            #[serde(skip_serializing_if = "Option::is_none")]
            server_flags: Option<i32>,
            /// Id of server channel
            channel_id: String,
            /// Name of server channel
            channel_name: String,
            /// Description of server channel
            #[serde(skip_serializing_if = "Option::is_none")]
            channel_description: Option<String>,
            /// Name of user who created the invite
            user_name: String,
            /// Avatar of the user who created the invite
            #[serde(skip_serializing_if = "Option::is_none")]
            user_avatar: Option<File>,
            /// Number of members in this server
            member_count: i64,
        },
        /// Group channel invite
        Group {
            /// Invite code
            code: String,
            /// Id of group channel
            channel_id: String,
            /// Name of group channel
            channel_name: String,
            /// Description of group channel
            #[serde(skip_serializing_if = "Option::is_none")]
            channel_description: Option<String>,
            /// Name of user who created the invite
            user_name: String,
            /// Avatar of the user who created the invite
            #[serde(skip_serializing_if = "Option::is_none")]
            user_avatar: Option<File>,
        },
    }

    /// Invite join response
    #[serde(tag = "type")]
    #[allow(clippy::large_enum_variant)]
    pub enum InviteJoinResponse {
        Server {
            /// Channels in the server
            channels: Vec<Channel>,
            /// Server we are joining
            server: Server,
        },
        Group {
            /// Group channel we are joining
            channel: Channel,
            /// Members of this group
            users: Vec<User>,
        },
    }
);
