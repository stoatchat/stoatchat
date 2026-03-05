use iso8601_timestamp::{Duration, Timestamp};
use std::ops::Deref;

use nanoid::nanoid;
use revolt_result::Result;

use crate::{Database, MultiFactorAuthentication};

auto_derived_partial!(
    /// Multi-factor auth ticket
    pub struct MFATicket {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,

        /// Account Id
        pub account_id: String,

        /// Unique Token
        pub token: String,

        /// Whether this ticket has been validated
        /// (can be used for account actions)
        pub validated: bool,

        /// Whether this ticket is authorised
        /// (can be used to log a user in)
        pub authorised: bool,

        /// TOTP code at time of ticket creation
        pub last_totp_code: Option<String>,
    },
    "PartialMFATicket"
);

/// Ticket which is guaranteed to be valid for use
///
/// If used in a Rocket guard, it will be consumed on match
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatedTicket(pub MFATicket);

/// Ticket which is guaranteed to not be valid for use
#[derive(Debug, Serialize, Deserialize)]
pub struct UnvalidatedTicket(pub MFATicket);

impl MFATicket {
    /// Create a new MFA ticket
    pub fn new(account_id: String, validated: bool) -> MFATicket {
        MFATicket {
            id: ulid::Ulid::new().to_string(),
            account_id,
            token: nanoid!(64),
            validated,
            authorised: false,
            last_totp_code: None,
        }
    }

    /// Populate an MFA ticket with valid MFA codes
    pub async fn populate(&mut self, mfa: &MultiFactorAuthentication) {
        self.last_totp_code = mfa.totp_token.generate_code().ok();
    }

    /// Save model
    pub async fn save(&self, db: &Database) -> Result<()> {
        db.save_ticket(self).await
    }

    /// Check if this MFA ticket has expired
    pub fn is_expired(&self) -> bool {
        let now = Timestamp::now_utc();

        let datetime: Timestamp = ulid::Ulid::from_string(&self.id)
            .expect("Valid `ulid`")
            .datetime()
            .into();

        now > (datetime.checked_add(Duration::minutes(5)).unwrap())
    }

    /// Claim and remove this MFA ticket
    pub async fn claim(&self, db: &Database) -> Result<()> {
        if self.is_expired() {
            return Err(create_error!(InvalidToken));
        }

        db.delete_ticket(&self.id).await
    }
}

impl Deref for ValidatedTicket {
    type Target = MFATicket;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for UnvalidatedTicket {
    type Target = MFATicket;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
