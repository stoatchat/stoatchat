mod admin_audits;
mod admin_cases;
mod admin_comments;
mod admin_migrations;
mod admin_strikes;
mod admin_tokens;
mod admin_users;
mod bots;
mod channel_invites;
mod channel_unreads;
mod channel_webhooks;
mod channels;
mod emojis;
mod file_hashes;
mod files;
mod messages;
mod policy_changes;
mod ratelimit_events;
mod safety_reports;
mod safety_snapshots;
mod server_bans;
mod server_members;
mod servers;
mod user_settings;
mod users;

pub use admin_audits::*;
pub use admin_cases::*;
pub use admin_comments::*;
pub use admin_migrations::*;
pub use admin_strikes::*;
pub use admin_tokens::*;
pub use admin_users::*;
pub use bots::*;
pub use channel_invites::*;
pub use channel_unreads::*;
pub use channel_webhooks::*;
pub use channels::*;
pub use emojis::*;
pub use file_hashes::*;
pub use files::*;
pub use messages::*;
pub use policy_changes::*;
pub use ratelimit_events::*;
pub use safety_reports::*;
pub use safety_snapshots::*;
pub use server_bans::*;
pub use server_members::*;
pub use servers::*;
pub use user_settings::*;
pub use users::*;

use crate::{Database, ReferenceDb};

#[cfg(feature = "mongodb")]
use crate::MongoDb;

pub trait AbstractDatabase:
    Sync
    + Send
    + admin_audits::AbstractAdminAudits
    + admin_cases::AbstractAdminCases
    + admin_comments::AbstractAdminComments
    + admin_strikes::AbstractAdminStrikes
    + admin_users::AbstractAdminUsers
    + admin_tokens::AbstractAdminTokens
    + admin_migrations::AbstractMigrations
    + bots::AbstractBots
    + channels::AbstractChannels
    + channel_invites::AbstractChannelInvites
    + channel_unreads::AbstractChannelUnreads
    + channel_webhooks::AbstractWebhooks
    + emojis::AbstractEmojis
    + file_hashes::AbstractAttachmentHashes
    + files::AbstractAttachments
    + messages::AbstractMessages
    + policy_changes::AbstractPolicyChange
    + ratelimit_events::AbstractRatelimitEvents
    + safety_reports::AbstractReport
    + safety_snapshots::AbstractSnapshot
    + server_bans::AbstractServerBans
    + server_members::AbstractServerMembers
    + servers::AbstractServers
    + user_settings::AbstractUserSettings
    + users::AbstractUsers
{
}

impl AbstractDatabase for ReferenceDb {}

#[cfg(feature = "mongodb")]
impl AbstractDatabase for MongoDb {}

impl std::ops::Deref for Database {
    type Target = dyn AbstractDatabase;

    fn deref(&self) -> &Self::Target {
        match &self {
            Database::Reference(dummy) => dummy,
            #[cfg(feature = "mongodb")]
            Database::MongoDb(mongo) => mongo,
        }
    }
}
