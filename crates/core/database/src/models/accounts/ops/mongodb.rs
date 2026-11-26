use crate::{
    AbstractAccounts, Account, MongoDb, PasskeyCredential, util::webauthn::encode_credential_id,
};
use bson::{to_bson, to_document};
use iso8601_timestamp::{Duration, Timestamp};
use mongodb::options::{
    Collation, CollationStrength, FindOneAndUpdateOptions, FindOneOptions, ReturnDocument,
    UpdateOptions,
};
use revolt_result::Result;

const COL: &str = "accounts";

#[async_trait]
impl AbstractAccounts for MongoDb {
    /// Find account by id
    async fn fetch_account(&self, id: &str) -> Result<Account> {
        query!(self, find_one_by_id, COL, id)?.ok_or_else(|| create_error!(UnknownUser))
    }

    /// Find account by normalised email
    async fn fetch_account_by_normalised_email(
        &self,
        normalised_email: &str,
    ) -> Result<Option<Account>> {
        query!(
            self,
            find_one_with_options,
            COL,
            doc! {
                "email_normalised": normalised_email
            },
            FindOneOptions::builder()
                .collation(
                    Collation::builder()
                        .locale("en")
                        .strength(CollationStrength::Secondary)
                        .build(),
                )
                .build()
        )
    }

    /// Find account with active pending email verification
    async fn fetch_account_with_email_verification(&self, token: &str) -> Result<Account> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "verification.token": token,
                "verification.expiry": {
                    "$gte": to_bson(&Timestamp::now_utc()).unwrap()
                }
            }
        )?
        .ok_or_else(|| create_error!(InvalidToken))
    }

    /// Find account with active password reset
    async fn fetch_account_with_password_reset(&self, token: &str) -> Result<Account> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "password_reset.token": token,
                "password_reset.expiry": {
                    "$gte": to_bson(&Timestamp::now_utc()).unwrap()
                }
            }
        )?
        .ok_or_else(|| create_error!(InvalidToken))
    }

    /// Find account with active deletion token
    async fn fetch_account_with_deletion_token(&self, token: &str) -> Result<Account> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "deletion.token": token,
                "deletion.expiry": {
                    "$gte": to_bson(&Timestamp::now_utc()).unwrap()
                }
            }
        )?
        .ok_or_else(|| create_error!(InvalidToken))
    }

    /// Find account owning a passkey by its WebAuthn credential id
    async fn fetch_account_by_passkey(&self, credential_id: &[u8]) -> Result<Option<Account>> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "mfa.passkeys.credential_id": encode_credential_id(credential_id)
            }
        )
    }

    /// Find accounts which are due to be deleted
    async fn fetch_accounts_due_for_deletion(&self) -> Result<Vec<Account>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "deletion.status": "Scheduled",
                "deletion.after": {
                    "$lte": to_bson(&Timestamp::now_utc()).unwrap()
                }
            }
        )
    }

    /// Add a newly registered passkey
    async fn add_passkey(&self, account_id: &str, passkey: &PasskeyCredential) -> Result<()> {
        let result = self
            .col::<Account>(COL)
            .update_one(
                doc! {
                    "_id": account_id
                },
                doc! {
                    "$push": {
                        "mfa.passkeys": to_bson(passkey)
                            .map_err(|_| create_database_error!("to_bson", COL))?
                    }
                },
            )
            .await
            .map_err(|err| match *err.kind {
                mongodb::error::ErrorKind::Write(mongodb::error::WriteFailure::WriteError(
                    mongodb::error::WriteError { code: 11000, .. },
                )) => create_error!(PasskeyAlreadyRegistered),
                _ => create_database_error!("update_one", COL),
            })?;

        if result.matched_count == 0 {
            return Err(create_error!(UnknownUser));
        }

        Ok(())
    }

    /// Save a passkey's credential and last use after it has been used to authenticate
    async fn save_passkey(&self, account_id: &str, passkey: &PasskeyCredential) -> Result<()> {
        let credential = serde_json::to_value(&passkey.credential)
            .ok()
            .and_then(|credential| to_bson(&credential).ok())
            .ok_or_else(|| create_database_error!("to_bson", COL))?;

        let result = self
            .col::<Account>(COL)
            .update_one(
                doc! {
                    "_id": account_id,
                    "mfa.passkeys.id": &passkey.id
                },
                doc! {
                    "$set": {
                        "mfa.passkeys.$.credential": credential,
                        "mfa.passkeys.$.last_used": to_bson(&passkey.last_used)
                            .map_err(|_| create_database_error!("to_bson", COL))?
                    }
                },
            )
            .await
            .map_err(|_| create_database_error!("update_one", COL))?;

        if result.matched_count == 0 {
            return Err(create_error!(InvalidCredentials));
        }

        Ok(())
    }

    // Save account
    async fn save_account(&self, account: &Account) -> Result<()> {
        self.col::<Account>(COL)
            .update_one(
                doc! {
                    "_id": &account.id
                },
                doc! {
                    "$set": to_document(account).map_err(|_| create_database_error!("to_document", COL))?
                },
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await
            .map_err(|_| create_database_error!("find_one", COL))
            .map(|_| ())
    }

    async fn bump_lockout_count(&self, id: &str) -> Result<Account> {
        let now = Timestamp::now_utc();

        self.col::<Account>(COL)
            .find_one_and_update(
                doc! {
                    "_id": id
                },
                vec![
                    doc! {
                        "$set": {
                            "lockout.attempts": {
                                "$add": [{ "$ifNull": ["$lockout.attempts", 0] }, 1]
                            }
                        }
                    },
                    doc! {
                        "$set": {
                            "lockout.expiry": {
                                "$switch": {
                                    "branches": [
                                        {
                                            "case": { "$gte": ["$lockout.attempts", 5] },
                                            "then": to_bson(&(now + Duration::hours(1))).expect("bson time error")
                                        },
                                        {
                                            "case": { "$eq": ["$lockout.attempts", 4] },
                                            "then": to_bson(&(now + Duration::minutes(5))).expect("bson time error")
                                        },
                                        {
                                            "case": { "$eq": ["$lockout.attempts", 3] },
                                            "then": to_bson(&(now + Duration::minutes(1))).expect("bson time error")
                                        }
                                    ],
                                    "default": null
                                }
                            }
                        }
                    },
                ],
            )
            .with_options(
                FindOneAndUpdateOptions::builder()
                    .return_document(ReturnDocument::After)
                    .build(),
            )
            .await
            .map_err(|_| create_database_error!("find_one_and_update", COL))?
            .ok_or_else(|| create_error!(UnknownUser))
    }
}
