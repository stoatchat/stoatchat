use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};

use revolt_result::{Error, Result};

use crate::{Database, MFATicket, UnvalidatedTicket, ValidatedTicket};

#[async_trait]
impl<S> FromRequestParts<S> for MFATicket
where
    Database: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        let db = Database::from_ref(state);

        if let Some(Ok(token)) = parts.headers.get("x-mfa-ticket").map(|v| v.to_str()) {
            db.fetch_ticket_by_token(token).await
        } else {
            Err(create_error!(MissingHeaders))
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for ValidatedTicket
where
    Database: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        let db = Database::from_ref(state);

        let ticket = MFATicket::from_request_parts(parts, state).await?;

        if ticket.validated && ticket.claim(&db).await.is_ok() {
            Ok(ValidatedTicket(ticket))
        } else {
            Err(create_error!(InvalidToken))
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for UnvalidatedTicket
where
    Database: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        let ticket = MFATicket::from_request_parts(parts, state).await?;

        if !ticket.validated {
            Ok(UnvalidatedTicket(ticket))
        } else {
            Err(create_error!(InvalidToken))
        }
    }
}
