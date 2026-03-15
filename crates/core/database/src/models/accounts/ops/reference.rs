use crate::{AbstractAccounts, Account, DeletionInfo, EmailVerification, ReferenceDb};
use revolt_result::Result;

#[async_trait]
impl AbstractAccounts for ReferenceDb {
    /// Find account by id
    async fn fetch_account(&self, id: &str) -> Result<Account> {
        let accounts = self.accounts.lock().await;
        accounts
            .get(id)
            .cloned()
            .ok_or_else(|| create_error!(UnknownUser))
    }

    /// Find account by normalised email
    async fn fetch_account_by_normalised_email(
        &self,
        normalised_email: &str,
    ) -> Result<Option<Account>> {
        let accounts = self.accounts.lock().await;
        Ok(accounts
            .values()
            .find(|account| account.email_normalised == normalised_email)
            .cloned())
    }

    /// Find account with active pending email verification
    async fn fetch_account_with_email_verification(&self, token_to_match: &str) -> Result<Account> {
        let accounts = self.accounts.lock().await;
        accounts
            .values()
            .find(|account| match &account.verification {
                EmailVerification::Pending { token, .. }
                | EmailVerification::Moving { token, .. } => token == token_to_match,
                _ => false,
            })
            .cloned()
            .ok_or_else(|| create_error!(InvalidToken))
    }

    /// Find account with active password reset
    async fn fetch_account_with_password_reset(&self, token: &str) -> Result<Account> {
        let accounts = self.accounts.lock().await;
        accounts
            .values()
            .find(|account| {
                if let Some(reset) = &account.password_reset {
                    reset.token == token
                } else {
                    false
                }
            })
            .cloned()
            .ok_or_else(|| create_error!(InvalidToken))
    }

    /// Find account with active deletion token
    async fn fetch_account_with_deletion_token(&self, token_to_match: &str) -> Result<Account> {
        let accounts = self.accounts.lock().await;
        accounts
            .values()
            .find(|account| {
                if let Some(DeletionInfo::WaitingForVerification { token, .. }) = &account.deletion
                {
                    token == token_to_match
                } else {
                    false
                }
            })
            .cloned()
            .ok_or_else(|| create_error!(InvalidToken))
    }

    /// Find accounts which are due to be deleted
    async fn fetch_accounts_due_for_deletion(&self) -> Result<Vec<Account>> {
        let accounts = self.accounts.lock().await;
        Ok(accounts
            .values()
            .filter(|account| {
                if let Some(DeletionInfo::Scheduled { .. }) = &account.deletion {
                    unimplemented!()
                } else {
                    false
                }
            })
            .cloned()
            .collect())
    }

    // Save account
    async fn save_account(&self, account: &Account) -> Result<()> {
        let mut accounts = self.accounts.lock().await;
        accounts.insert(account.id.to_string(), account.clone());
        Ok(())
    }
}
