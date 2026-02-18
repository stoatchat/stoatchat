use crate::{Database, Session};
use revolt_result::Error;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Session {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(token) = request.headers().get("x-session-token").next() {
            if let Ok(session) = request
                .rocket()
                .state::<Database>()
                .expect("`Database`")
                .fetch_session_by_token(token)
                .await
            {
                Outcome::Success(session)
            } else {
                Outcome::Error((Status::Unauthorized, create_error!(InvalidSession)))
            }
        } else {
            Outcome::Error((Status::Unauthorized, create_error!(MissingHeaders)))
        }
    }
}
