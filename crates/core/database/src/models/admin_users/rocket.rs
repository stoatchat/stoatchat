use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};
use revolt_result::Error;
use crate::{AdminUser, Database};

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminUser {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let user: &Option<AdminUser> = request
            .local_cache_async(async {
                let db = request.rocket().state::<Database>().expect("`Database`");

                let token = request
                    .headers()
                    .get("x-admin-user")
                    .next()
                    .map(|x| x.to_string());

                if let Some(token) = token {
                    if let Ok(admin_token) = db.admin_token_authenticate(&token).await {
                        if let Ok(user) = db.admin_user_fetch(&admin_token.user_id).await {
                            return Some(user);
                        }
                    }
                }

                None
            })
            .await;

        if let Some(user) = user {
            if !user.active {
                Outcome::Error((Status::Unauthorized, create_error!(LockedOut)))
            } else {
                Outcome::Success(user.clone())
            }
        } else {
            Outcome::Error((Status::Unauthorized, create_error!(InvalidCredentials)))
        }
    }
}
