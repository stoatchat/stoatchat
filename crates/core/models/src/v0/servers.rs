use super::{Channel, File, RE_COLOUR};

use revolt_permissions::{Override, OverrideField};
use std::collections::HashMap;

#[cfg(feature = "validator")]
use validator::Validate;

#[cfg(feature = "rocket")]
use rocket::FromForm;

auto_derived_partial!(
    /// Server
    pub struct Server {
        /// Unique Id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: String,
        /// User id of the owner
        pub owner: String,

        /// Name of the server
        pub name: String,
        /// Description for the server
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub description: Option<String>,

        /// Channels within this server
        // TODO: investigate if this is redundant and can be removed
        pub channels: Vec<String>,
        /// Categories for this server
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub categories: Option<Vec<Category>>,
        /// Configuration for sending system event messages
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub system_messages: Option<SystemMessageChannels>,

        /// Roles for this server
        #[cfg_attr(
            feature = "serde",
            serde(
                default = "HashMap::<String, Role>::new",
                skip_serializing_if = "HashMap::<String, Role>::is_empty"
            )
        )]
        pub roles: HashMap<String, Role>,
        /// Default set of server and channel permissions
        pub default_permissions: i64,

        /// Icon attachment
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub icon: Option<File>,
        /// Banner attachment
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub banner: Option<File>,

        /// Bitfield of server flags
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_zero_u32", default)
        )]
        pub flags: u32,

        /// Whether this server is flagged as not safe for work
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_false", default)
        )]
        pub nsfw: bool,
        /// Whether to enable analytics
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_false", default)
        )]
        pub analytics: bool,
        /// Whether this server should be publicly discoverable
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_false", default)
        )]
        pub discoverable: bool,
    },
    "PartialServer"
);

auto_derived_partial!(
    /// Role
    pub struct Role {
        /// Unique Id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: String,
        /// Role name
        pub name: String,
        /// Permissions available to this role
        pub permissions: OverrideField,
        /// Colour used for this role
        ///
        /// This can be any valid CSS colour
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub colour: Option<String>,
        /// Whether this role should be shown separately on the member sidebar
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_false", default)
        )]
        pub hoist: bool,
        /// Ranking of this role
        #[cfg_attr(feature = "serde", serde(default))]
        pub rank: i64,
    },
    "PartialRole"
);

auto_derived!(
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

    /// Channel category
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct Category {
        /// Unique ID for this category
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub id: String,
        /// Title for this category
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub title: String,
        /// Channels in this category
        pub channels: Vec<String>,
    }

    /// System message channel assignments
    pub struct SystemMessageChannels {
        /// ID of channel to send user join messages in
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub user_joined: Option<String>,
        /// ID of channel to send user left messages in
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub user_left: Option<String>,
        /// ID of channel to send user kicked messages in
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub user_kicked: Option<String>,
        /// ID of channel to send user banned messages in
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub user_banned: Option<String>,
    }

    /// Information about new server to create
    #[derive(Default)]
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct DataCreateServer {
        /// Server name
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub name: String,
        /// Server description
        #[cfg_attr(feature = "validator", validate(length(min = 0, max = 1024)))]
        pub description: Option<String>,
        /// Whether this server is age-restricted
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub nsfw: Option<bool>,
    }

    /// Information about new role to create
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct DataCreateRole {
        /// Role name
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub name: String,
        /// Ranking position
        ///
        /// Smaller values take priority.
        ///
        /// **Removed** - no effect, use the edit server role positions route
        pub rank: Option<i64>,
    }

    /// Response after creating new role
    // TODO: remove this in favor of just Role
    pub struct NewRoleResponse {
        /// Id of the role
        pub id: String,
        /// New role
        pub role: Role,
    }

    /// Information returned when creating server
    pub struct CreateServerLegacyResponse {
        /// Server object
        pub server: Server,
        /// Default channels
        pub channels: Vec<Channel>,
    }

    /// Options when fetching server
    #[cfg_attr(feature = "rocket", derive(FromForm))]
    pub struct OptionsFetchServer {
        /// Whether to include channels
        pub include_channels: Option<bool>,
    }

    /// Fetch server information
    #[serde(untagged)]
    pub enum FetchServerResponse {
        JustServer(Server),
        ServerWithChannels {
            #[serde(flatten)]
            server: Server,
            channels: Vec<Channel>,
        },
    }

    /// New server information
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct DataEditServer {
        /// Server name
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub name: Option<String>,
        /// Server description
        #[cfg_attr(feature = "validator", validate(length(min = 0, max = 1024)))]
        pub description: Option<String>,

        /// Attachment Id for icon
        pub icon: Option<String>,
        /// Attachment Id for banner
        pub banner: Option<String>,

        /// Category structure for server
        #[cfg_attr(feature = "validator", validate)]
        pub categories: Option<Vec<Category>>,
        /// System message configuration
        pub system_messages: Option<SystemMessageChannels>,

        /// Bitfield of server flags
        #[cfg_attr(feature = "validator", serde(skip_serializing_if = "Option::is_none"))]
        pub flags: Option<i32>,

        // Whether this server is age-restricted
        // nsfw: Option<bool>,
        /// Whether this server is public and should show up on [Revolt Discover](https://rvlt.gg)
        pub discoverable: Option<bool>,
        /// Whether analytics should be collected for this server
        ///
        /// Must be enabled in order to show up on [Revolt Discover](https://rvlt.gg).
        pub analytics: Option<bool>,

        /// Fields to remove from server object
        #[cfg_attr(feature = "serde", serde(default))]
        pub remove: Vec<FieldsServer>,
    }

    /// New role information
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct DataEditRole {
        /// Role name
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub name: Option<String>,
        /// Role colour
        #[cfg_attr(
            feature = "validator",
            validate(length(min = 1, max = 128), regex = "RE_COLOUR")
        )]
        pub colour: Option<String>,
        /// Whether this role should be displayed separately
        pub hoist: Option<bool>,
        /// Ranking position
        ///
        /// **Removed** - no effect, use the edit server role positions route
        pub rank: Option<i64>,
        /// Fields to remove from role object
        #[cfg_attr(feature = "serde", serde(default))]
        pub remove: Vec<FieldsRole>,
    }

    /// New role permissions
    pub struct DataSetServerRolePermission {
        /// Allow / deny values for the role in this server.
        pub permissions: Override,
    }

    /// Options when leaving a server
    #[cfg_attr(feature = "rocket", derive(FromForm))]
    pub struct OptionsServerDelete {
        /// Whether to not send a leave message
        pub leave_silently: Option<bool>,
    }

    /// New role positions
    pub struct DataEditRoleRanks {
        pub ranks: Vec<String>,
    }
);
