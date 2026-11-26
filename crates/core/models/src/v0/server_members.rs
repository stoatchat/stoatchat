use std::collections::HashMap;

use super::{File, Role, User};

use iso8601_timestamp::Timestamp;
use once_cell::sync::Lazy;
use regex::Regex;

#[cfg(feature = "validator")]
use validator::Validate;

#[cfg(feature = "rocket")]
use rocket::FromForm;

/// Regex for valid role colours
///
/// Allows the use of named colours, rgb(a), variables and all gradients.
///
/// Flags:
/// - Case-insensitive (`i`)
///
/// Source:
/// ```regex
/// VALUE = [a-z ]+|var\(--[a-z\d-]+\)|rgba?\([\d, ]+\)|#[a-f0-9]+
/// ADDITIONAL_VALUE = \d+deg
/// COLOUR_SPACE = in [a-z-]+([ ]+(shorter|longer|increasing|decreasing)[ ]+hue)?
/// STOP = ([ ]+(\d{1,3}%|0))?
///
/// ^(?:VALUE|(repeating-)?(linear|conic|radial)-gradient\((VALUE|ADDITIONAL_VALUE)(COLOUR_SPACE)?STOP(,[ ]*(VALUE)STOP)+\))$
/// ```
pub static RE_COLOUR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(?:[a-z ]+|var\(--[a-z\d-]+\)|rgba?\([\d, ]+\)|#[a-f0-9]+|(repeating-)?(linear|conic|radial)-gradient\(([a-z ]+|var\(--[a-z\d-]+\)|rgba?\([\d, ]+\)|#[a-f0-9]+|\d+deg)([ ]+in[ ]+[a-z-]+([ ]+(shorter|longer|increasing|decreasing)[ ]+hue)?)?([ ]+(\d{1,3}%|0))?(,[ ]*([a-z ]+|var\(--[a-z\d-]+\)|rgba?\([\d, ]+\)|#[a-f0-9]+)([ ]+(\d{1,3}%|0))?)+\))$").unwrap()
});

fn default_true() -> bool {
    true
}

fn is_true(x: &bool) -> bool {
    *x
}

auto_derived_partial!(
    /// Server Member
    pub struct Member {
        /// Unique member id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: MemberCompositeKey,

        /// Time at which this user joined the server
        pub joined_at: Timestamp,

        /// Member's nickname
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub nickname: Option<String>,
        /// Member's pronouns
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub pronouns: Option<String>,
        /// Avatar attachment
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub avatar: Option<File>,

        /// Member's roles
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "Vec::is_empty", default)
        )]
        pub roles: Vec<String>,
        /// Timestamp this member is timed out until
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub timeout: Option<Timestamp>,

        /// Whether the member is server-wide voice muted
        #[serde(skip_serializing_if = "is_true", default = "default_true")]
        pub can_publish: bool,
        /// Whether the member is server-wide voice deafened
        #[serde(skip_serializing_if = "is_true", default = "default_true")]
        pub can_receive: bool,
    },
    "PartialMember"
);

auto_derived!(
    /// Composite primary key consisting of server and user id
    #[derive(Hash, Default)]
    pub struct MemberCompositeKey {
        /// Server Id
        pub server: String,
        /// User Id
        pub user: String,
    }

    /// Optional fields on server member object
    pub enum FieldsMember {
        Nickname,
        Pronouns,
        Avatar,
        Roles,
        Timeout,
        CanReceive,
        CanPublish,
        JoinedAt,
        VoiceChannel,
    }

    /// Member removal intention
    pub enum RemovalIntention {
        Leave,
        Kick,
        Ban,
    }

    /// Member response
    #[serde(untagged)]
    pub enum MemberResponse {
        Member(Member),
        MemberWithRoles {
            member: Member,
            roles: HashMap<String, Role>,
        },
    }

    /// Options for fetching all members
    #[cfg_attr(feature = "rocket", derive(FromForm))]
    pub struct OptionsFetchAllMembers {
        /// Whether to exclude offline users
        pub exclude_offline: Option<bool>,
    }

    /// Response with all members
    pub struct AllMemberResponse {
        /// List of members
        pub members: Vec<Member>,
        /// List of users
        pub users: Vec<User>,
    }

    /// Member search options
    #[cfg_attr(feature = "validator", derive(Validate))]
    #[cfg_attr(feature = "rocket", derive(FromForm))]
    pub struct OptionsMemberSearch {
        /// Search string; matches nickname, display name or username (supports username#1234)
        /// usernames are 32 characters long and #1234 is 5 characters adding up to a maximum of 37.
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 37)))]
        pub query: Option<String>,
        /// Search with roles, the member must have all the roles provided,
        /// 10 is a sane limit for amount of roles you should use to search
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 10)))]
        pub roles: Option<Vec<String>>,
        /// Return members after this user id
        #[cfg_attr(feature = "validator", validate(length(min = 26, max = 26)))]
        pub after: Option<String>,
        /// Maximum number of members to return, defaults to 25
        #[cfg_attr(feature = "validator", validate(range(min = 1, max = 100)))]
        pub limit: Option<i64>,
    }

    /// Member search results
    pub struct MemberSearchResponse {
        /// List of matching members
        pub members: Vec<Member>,
        /// Users of the matching members
        pub users: Vec<User>,
        /// Cursor to pass as `after` for the next page.
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub next: Option<String>,
    }

    /// New member information
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct DataMemberEdit {
        /// Member nickname
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub nickname: Option<String>,
        /// Member pronouns
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 24)))]
        pub pronouns: Option<String>,
        /// Attachment Id to set for avatar
        pub avatar: Option<String>,
        /// Array of role ids
        pub roles: Option<Vec<String>>,
        /// Timestamp this member is timed out until
        pub timeout: Option<Timestamp>,
        /// server-wide voice muted
        pub can_publish: Option<bool>,
        /// server-wide voice deafened
        pub can_receive: Option<bool>,
        /// voice channel to move to if already in a voice channel
        pub voice_channel: Option<String>,
        /// Fields to remove from channel object
        #[cfg_attr(feature = "serde", serde(default))]
        pub remove: Vec<FieldsMember>,
    }
);
