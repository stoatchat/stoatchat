use crate::{AbstractAccountInvites, AccountInvite, MongoDb};
use bson::to_document;
use mongodb::options::UpdateOptions;
use revolt_result::Result;

// TODO: rename this to "account_invites"
const COL: &str = "invites";

#[async_trait]
impl AbstractAccountInvites for MongoDb {
    /// Find invite by id
    async fn fetch_account_invite(&self, id: &str) -> Result<AccountInvite> {
        self.find_one_by_id(COL, id)
            .await
            .map_err(|_| create_database_error!("find_one", COL))?
            .ok_or_else(|| create_error!(InvalidInvite))
    }

    /// Save invite
    async fn save_account_invite(&self, invite: &AccountInvite) -> Result<()> {
        self.col::<AccountInvite>(COL)
            .update_one(
                doc! {
                    "_id": &invite.id
                },
                doc! {
                    "$set": to_document(invite).map_err(|_| create_database_error!("to_document", COL))?,
                },
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await
            .map_err(|_| create_database_error!("upsert_one", COL))
            .map(|_| ())
    }
}
