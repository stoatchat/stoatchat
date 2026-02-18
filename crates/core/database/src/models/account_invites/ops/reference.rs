use crate::{AbstractAccountInvites, AccountInvite, ReferenceDb};
use revolt_result::Result;

#[async_trait]
impl AbstractAccountInvites for ReferenceDb {
    /// Find invite by id
    async fn fetch_account_invite(&self, id: &str) -> Result<AccountInvite> {
        let invites = self.account_invites.lock().await;
        invites
            .get(id)
            .cloned()
            .ok_or_else(|| create_error!(InvalidInvite))
    }

    /// Save invite
    async fn save_account_invite(&self, invite: &AccountInvite) -> Result<()> {
        let mut invites = self.account_invites.lock().await;
        invites.insert(invite.id.to_string(), invite.clone());
        Ok(())
    }
}
