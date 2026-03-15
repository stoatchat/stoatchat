use crate::{Account, Database, Session};
use revolt_result::Error;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Account {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<Session>().await {
            Outcome::Success(session) => {
                if let Ok(account) = request
                    .rocket()
                    .state::<Database>()
                    .expect("`Database`")
                    .fetch_account(&session.user_id)
                    .await
                {
                    Outcome::Success(account)
                } else {
                    Outcome::Error((Status::InternalServerError, create_error!(InternalError)))
                }
            }
            Outcome::Forward(_) => unreachable!(),
            Outcome::Error(err) => Outcome::Error(err),
        }
    }
}
