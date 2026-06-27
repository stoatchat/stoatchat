use revolt_result::{create_error, Error};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

pub struct AuthHeader<'a>(&'a str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthHeader<'r> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.headers().get("Authorization").next() {
            Some(token) => Outcome::Success(Self(token)),
            None => Outcome::Error((Status::Unauthorized, create_error!(NotAuthenticated))),
        }
    }
}

impl std::ops::Deref for AuthHeader<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
