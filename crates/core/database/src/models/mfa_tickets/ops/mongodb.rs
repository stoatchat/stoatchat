use std::time::Duration;

use crate::{AbstractMFATickets, MFATicket, MongoDb};
use bson::to_document;
use iso8601_timestamp::Timestamp;
use mongodb::options::UpdateOptions;
use revolt_result::Result;
use ulid::Ulid;

const COL: &str = "mfa_tickets";

#[async_trait]
impl AbstractMFATickets for MongoDb {
    /// Find ticket by token
    ///
    /// Ticket is only valid for 1 minute
    async fn fetch_ticket_by_token(&self, token: &str) -> Result<Option<MFATicket>> {
        let ticket: MFATicket = self
            .col(COL)
            .find_one(doc! {
                "token": token
            })
            .await
            .map_err(|_| create_database_error!("find_one", COL))?
            .ok_or_else(|| create_error!(InvalidToken))?;

        if let Ok(ulid) = Ulid::from_string(&ticket.id) {
            if Timestamp::from(ulid.datetime() + Duration::from_mins(1)) > Timestamp::now_utc() {
                Ok(Some(ticket))
            } else {
                Err(create_error!(InvalidToken))
            }
        } else {
            Err(create_error!(InvalidToken))
        }
    }

    /// Save ticket
    async fn save_ticket(&self, ticket: &MFATicket) -> Result<()> {
        self.col::<MFATicket>(COL)
            .update_one(
                doc! {
                    "_id": &ticket.id
                },
                doc! {
                    "$set": to_document(ticket).map_err(|_| create_database_error!("to_document", COL))?,
                },
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await
            .map_err(|_| create_database_error!("upsert_one", COL))
            .map(|_| ())
    }

    /// Delete ticket
    async fn delete_ticket(&self, id: &str) -> Result<()> {
        self.col::<MFATicket>(COL)
            .delete_one(doc! {
                "_id": id
            })
            .await
            .map_err(|_| create_database_error!("delete_one", COL))
            .map(|_| ())
    }
}
