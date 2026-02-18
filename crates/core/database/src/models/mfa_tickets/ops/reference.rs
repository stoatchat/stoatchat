use crate::{AbstractMFATickets, MFATicket, ReferenceDb};
use revolt_result::Result;

#[async_trait]
impl AbstractMFATickets for ReferenceDb {
    /// Find ticket by token
    async fn fetch_ticket_by_token(&self, token: &str) -> Result<Option<MFATicket>> {
        let tickets = self.tickets.lock().await;
        Ok(tickets
            .values()
            .find(|ticket| ticket.token == token)
            .cloned())
    }

    /// Save ticket
    async fn save_ticket(&self, ticket: &MFATicket) -> Result<()> {
        let mut tickets = self.tickets.lock().await;
        tickets.insert(ticket.id.to_string(), ticket.clone());
        Ok(())
    }

    /// Delete ticket
    async fn delete_ticket(&self, id: &str) -> Result<()> {
        let mut tickets = self.tickets.lock().await;
        if tickets.remove(id).is_some() {
            Ok(())
        } else {
            Err(create_error!(InvalidToken))
        }
    }
}
