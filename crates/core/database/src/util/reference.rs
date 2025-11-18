use std::str::FromStr;

use revolt_result::Result;
#[cfg(feature = "rocket-impl")]
use rocket::request::FromParam;

use crate::{
    Bot, Channel, Database, Emoji, Invite, Member, Message, Server, ServerBan, User, Webhook,
};

/// Reference to some object in the database
pub struct Reference<'a> {
    /// Id of object
    pub id: &'a str,
}

impl<'a> Reference<'a> {
    /// Create a Ref from an unchecked string
    pub fn from_unchecked(id: &'a str) -> Reference<'a> {
        Reference { id }
    }

    /// Fetch ban from Ref
    pub async fn as_ban(&self, db: &Database, server: &str) -> Result<ServerBan> {
        db.fetch_ban(server, self.id).await
    }

    /// Fetch bot from Ref
    pub async fn as_bot(&self, db: &Database) -> Result<Bot> {
        db.fetch_bot(self.id).await
    }

    /// Fetch emoji from Ref
    pub async fn as_emoji(&self, db: &Database) -> Result<Emoji> {
        db.fetch_emoji(self.id).await
    }

    /// Fetch channel from Ref
    pub async fn as_channel(&self, db: &Database) -> Result<Channel> {
        db.fetch_channel(self.id).await
    }

    /// Fetch invite from Ref or create invite to server if discoverable
    pub async fn as_invite(&self, db: &Database) -> Result<Invite> {
        if ulid::Ulid::from_str(self.id).is_ok() {
            let server = self.as_server(db).await?;
            if !server.discoverable {
                return Err(create_error!(NotFound));
            }

            Ok(Invite::Server {
                code: self.id.to_string(),
                server: server.id,
                creator: server.owner,
                channel: server
                    .channels
                    .into_iter()
                    .next()
                    .ok_or(create_error!(NotFound))?,
            })
        } else {
            db.fetch_invite(self.id).await
        }
    }

    /// Fetch message from Ref
    pub async fn as_message(&self, db: &Database) -> Result<Message> {
        db.fetch_message(self.id).await
    }

    /// Fetch message from Ref and validate channel
    pub async fn as_message_in_channel(&self, db: &Database, channel: &str) -> Result<Message> {
        let msg = db.fetch_message(self.id).await?;
        if msg.channel != channel {
            return Err(create_error!(NotFound));
        }

        Ok(msg)
    }

    /// Fetch member from Ref
    pub async fn as_member(&self, db: &Database, server: &str) -> Result<Member> {
        db.fetch_member(server, self.id).await
    }

    /// Fetch server from Ref
    pub async fn as_server(&self, db: &Database) -> Result<Server> {
        db.fetch_server(self.id).await
    }

    /// Fetch user from Ref
    pub async fn as_user(&self, db: &Database) -> Result<User> {
        db.fetch_user(self.id).await
    }

    /// Fetch webhook from Ref
    pub async fn as_webhook(&self, db: &Database) -> Result<Webhook> {
        db.fetch_webhook(self.id).await
    }
}

#[cfg(feature = "rocket-impl")]
impl<'r> FromParam<'r> for Reference<'r> {
    type Error = &'r str;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Ok(Reference::from_unchecked(param))
    }
}
