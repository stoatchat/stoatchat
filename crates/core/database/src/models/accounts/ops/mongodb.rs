use crate::{AbstractAccounts, Account, MongoDb};
use bson::{to_bson, to_document};
use futures::stream::TryStreamExt;
use iso8601_timestamp::Timestamp;
use mongodb::options::{Collation, CollationStrength, FindOneOptions, UpdateOptions};
use revolt_result::Result;

const COL: &str = "accounts";

#[async_trait]
impl AbstractAccounts for MongoDb {
    /// Find account by id
    async fn fetch_account(&self, id: &str) -> Result<Account> {
        self.find_one_by_id(COL, id)
            .await
            .map_err(|_| create_database_error!("find_one", "accounts"))?
            .ok_or_else(|| create_error!(UnknownUser))
    }

    /// Find account by normalised email
    async fn fetch_account_by_normalised_email(
        &self,
        normalised_email: &str,
    ) -> Result<Option<Account>> {
        self.col(COL)
            .find_one(doc! {
                "email_normalised": normalised_email
            })
            .with_options(
                FindOneOptions::builder()
                    .collation(
                        Collation::builder()
                            .locale("en")
                            .strength(CollationStrength::Secondary)
                            .build(),
                    )
                    .build(),
            )
            .await
            .map_err(|_| create_database_error!("find_one", "accounts"))
    }

    /// Find account with active pending email verification
    async fn fetch_account_with_email_verification(&self, token: &str) -> Result<Account> {
        self.col("accounts")
            .find_one(doc! {
                "verification.token": token,
                "verification.expiry": {
                    "$gte": to_bson(&Timestamp::now_utc()).unwrap()
                }
            })
            .await
            .map_err(|_| create_database_error!("find_one", "accounts"))?
            .ok_or_else(|| create_error!(InvalidToken))
    }

    /// Find account with active password reset
    async fn fetch_account_with_password_reset(&self, token: &str) -> Result<Account> {
        self.col(COL)
            .find_one(doc! {
                "password_reset.token": token,
                "password_reset.expiry": {
                    "$gte": to_bson(&Timestamp::now_utc()).unwrap()
                }
            })
            .await
            .map_err(|_| create_database_error!("find_one", "accounts"))?
            .ok_or_else(|| create_error!(InvalidToken))
    }

    /// Find account with active deletion token
    async fn fetch_account_with_deletion_token(&self, token: &str) -> Result<Account> {
        self.col(COL)
            .find_one(doc! {
                "deletion.token": token,
                "deletion.expiry": {
                    "$gte": to_bson(&Timestamp::now_utc()).unwrap()
                }
            })
            .await
            .map_err(|_| create_database_error!("find_one", "accounts"))?
            .ok_or_else(|| create_error!(InvalidToken))
    }

    /// Find accounts which are due to be deleted
    async fn fetch_accounts_due_for_deletion(&self) -> Result<Vec<Account>> {
        self.col("accounts")
            .find(doc! {
                "deletion.status": "Scheduled",
                "deletion.after": {
                    "$lte": to_bson(&Timestamp::now_utc()).unwrap()
                }
            })
            .await
            .map_err(|_| create_database_error!("find", "accounts"))?
            .try_collect()
            .await
            .map_err(|_| create_database_error!("collect", "accounts"))
    }

    // Save account
    async fn save_account(&self, account: &Account) -> Result<()> {
        self.col::<Account>(COL)
            .update_one(
                doc! {
                    "_id": &account.id
                },
                doc! {
                    "$set": to_document(account).map_err(|_| create_database_error!("to_document", "accounts"))?
                },
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await
            .map_err(|_| create_database_error!("find_one", "accounts"))
            .map(|_| ())
    }
}
