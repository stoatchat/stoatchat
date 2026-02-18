use revolt_result::Result;

use crate::MFATicket;

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractMFATickets: Sync + Send {
    /// Find ticket by token
    async fn fetch_ticket_by_token(&self, token: &str) -> Result<Option<MFATicket>>;

    /// Save ticket
    async fn save_ticket(&self, ticket: &MFATicket) -> Result<()>;

    /// Delete ticket
    async fn delete_ticket(&self, id: &str) -> Result<()>;
}
