use iso8601_timestamp::Timestamp;
use revolt_result::Result;

use crate::Invite;
use crate::ReferenceDb;

use super::AbstractChannelInvites;

#[async_trait]
impl AbstractChannelInvites for ReferenceDb {
    /// Insert a new invite into the database
    async fn insert_invite(&self, invite: &Invite) -> Result<()> {
        let mut invites = self.channel_invites.lock().await;
        if invites.contains_key(invite.code()) {
            Err(create_database_error!("insert", "invite"))
        } else {
            invites.insert(invite.code().to_string(), invite.clone());
            Ok(())
        }
    }

    /// Fetch an invite by the code
    async fn fetch_invite(&self, code: &str) -> Result<Invite> {
        let invites = self.channel_invites.lock().await;
        invites
            .get(code)
            .cloned()
            .ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch all invites for a server
    async fn fetch_invites_for_server(&self, server_id: &str) -> Result<Vec<Invite>> {
        let invites = self.channel_invites.lock().await;
        Ok(invites
            .values()
            .filter(|invite| match invite {
                Invite::Server { server, .. } => server == server_id,
                _ => false,
            })
            .cloned()
            .collect())
    }

    /// Delete an invite by its code
    async fn delete_invite(&self, code: &str) -> Result<()> {
        let mut invites = self.channel_invites.lock().await;
        if invites.remove(code).is_some() {
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Atomically consume one use of an invite, returning the invite's state
    /// *after* the increment — or `None` if it had no uses remaining (or didn't exist).
    async fn consume_invite_use(&self, code: &str) -> Result<Option<Invite>> {
        let mut invites = self.channel_invites.lock().await;

        let Some(invite) = invites.get_mut(code) else {
            return Ok(None);
        };

        let (uses, max_uses) = match invite {
            Invite::Server { uses, max_uses, .. } | Invite::Group { uses, max_uses, .. } => {
                (uses, max_uses)
            }
        };

        if let Some(max_uses) = max_uses {
            if *uses >= *max_uses {
                return Ok(None);
            }
        }

        *uses += 1;
        Ok(Some(invite.clone()))
    }

    async fn fetch_expired_invites(&self) -> Result<Vec<Invite>> {
        let invites = self.channel_invites.lock().await;
        let now = Timestamp::now_utc();

        Ok(invites
            .values()
            .filter(|invite| {
                let expires = match invite {
                    Invite::Server { expires, .. } | Invite::Group { expires, .. } => expires,
                };
                matches!(expires, Some(expires) if *expires <= now)
            })
            .cloned()
            .collect())
    }
}
