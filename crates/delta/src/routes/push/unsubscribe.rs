use revolt_database::{Database, Session};
use revolt_result::{create_database_error, Result};
use rocket_empty::EmptyResponse;

use rocket::State;

/// # Unsubscribe
///
/// Remove the Web Push subscription associated with the current session.
#[openapi(tag = "Web Push")]
#[post("/unsubscribe")]
pub async fn unsubscribe(db: &State<Database>, mut session: Session) -> Result<EmptyResponse> {
    session.subscription = None;
    session
        .save(db)
        .await
        .map(|_| EmptyResponse)
        .map_err(|_| create_database_error!("save", "session"))
}
