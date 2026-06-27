use axum::{extract::{FromRef, FromRequestParts}, http::request::Parts};

use revolt_result::{Error, Result};

use crate::{Account, Database, Session};

#[async_trait]
impl<S> FromRequestParts<S> for Account
where
    Database: FromRef<S>,
    S: Send + Sync
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        let db = Database::from_ref(state);

        let session = Session::from_request_parts(parts, state).await?;

        db.fetch_account(&session.user_id).await
    }
}
