use revolt_result::{create_error, Result};

use crate::{Channel, Database, User};

static ALPHABET: [char; 54] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J',
    'K', 'M', 'N', 'P', 'Q', 'R', 'S', 'T', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f',
    'g', 'h', 'j', 'k', 'm', 'n', 'p', 'q', 'r', 's', 't', 'v', 'w', 'x', 'y', 'z',
];

auto_derived!(
    /// Invite
    #[serde(tag = "type")]
    pub enum Invite {
        /// Invite to a specific server channel
        Server {
            /// Invite code
            #[serde(rename = "_id")]
            code: String,
            /// Id of the server this invite points to
            server: String,
            /// Id of user who created this invite
            creator: String,
            /// Id of the server channel this invite points to
            channel: String,
        },
        /// Invite to a group channel
        Group {
            /// Invite code
            #[serde(rename = "_id")]
            code: String,
            /// Id of user who created this invite
            creator: String,
            /// Id of the group channel this invite points to
            channel: String,
        }, /* User {
               code: String,
               user: String
           } */
    }
);

#[allow(clippy::disallowed_methods)]
impl Invite {
    /// Get the invite code for this invite
    pub fn code(&'_ self) -> &'_ str {
        match self {
            Invite::Server { code, .. } | Invite::Group { code, .. } => code,
        }
    }

    /// Get the ID of the user who created this invite
    pub fn creator(&'_ self) -> &'_ str {
        match self {
            Invite::Server { creator, .. } | Invite::Group { creator, .. } => creator,
        }
    }

    /// Create a new invite from given information
    pub async fn create_channel_invite(
        db: &Database,
        creator: &User,
        channel: &Channel,
    ) -> Result<Invite> {
        let code = nanoid::nanoid!(8, &ALPHABET);
        let invite = match &channel {
            Channel::Group { id, .. } => Ok(Invite::Group {
                code,
                creator: creator.id.clone(),
                channel: id.clone(),
            }),
            Channel::TextChannel { id, server, .. } => {
                Ok(Invite::Server {
                    code,
                    creator: creator.id.clone(),
                    server: server.clone(),
                    channel: id.clone(),
                })
            }
            _ => Err(create_error!(InvalidOperation)),
        }?;

        db.insert_invite(&invite).await?;
        Ok(invite)
    }

    /// Resolve an invite by its ID or by a public server ID
    pub async fn find(db: &Database, code: &str) -> Result<Invite> {
        if let Ok(invite) = db.fetch_invite(code).await {
            return Ok(invite);
        } else if let Ok(server) = db.fetch_server(code).await {
            if server.discoverable {
                if let Some(channel) = server.channels.into_iter().next() {
                    return Ok(Invite::Server {
                        code: code.to_string(),
                        server: server.id,
                        creator: server.owner,
                        channel,
                    });
                }
            }
        }

        Err(create_error!(NotFound))
    }
}
