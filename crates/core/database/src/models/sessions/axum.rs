use axum::{extract::{FromRef, FromRequestParts}, http::request::Parts};

use revolt_result::{create_error, Error, Result};

use crate::{Database, Session};

#[async_trait]
impl<S> FromRequestParts<S> for Session
where
    Database: FromRef<S>,
    S: Send + Sync
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        let db = Database::from_ref(state);

        if let Some(Ok(token)) = parts.headers.get("x-session-token").map(|v| v.to_str()) {
            db.fetch_session_by_token(token).await
        } else {
            Err(create_error!(MissingHeaders))
        }
    }
}
