use crate::{Database, MFATicket, UnvalidatedTicket, ValidatedTicket};
use revolt_result::Error;
use rocket::{
    http::Status,
    outcome::Outcome,
    request::{self, FromRequest},
    Request,
};

#[rocket::async_trait]
impl<'r> FromRequest<'r> for MFATicket {
    type Error = Error;

    #[allow(clippy::collapsible_match)]
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        if let Some(header_mfa_ticket) = request.headers().get("x-mfa-ticket").next() {
            if let Ok(Some(ticket)) = request
                .rocket()
                .state::<Database>()
                .expect("`Database`")
                .fetch_ticket_by_token(header_mfa_ticket)
                .await
            {
                Outcome::Success(ticket)
            } else {
                Outcome::Error((Status::Unauthorized, create_error!(InvalidToken)))
            }
        } else {
            Outcome::Error((Status::Unauthorized, create_error!(MissingHeaders)))
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ValidatedTicket {
    type Error = Error;

    #[allow(clippy::collapsible_match)]
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match request.guard::<MFATicket>().await {
            Outcome::Success(ticket) => {
                if ticket.validated {
                    let db = request
                        .rocket()
                        .state::<Database>()
                        .expect("`Database`");

                    if ticket.claim(db).await.is_ok() {
                        Outcome::Success(ValidatedTicket(ticket))
                    } else {
                        Outcome::Error((Status::Forbidden, create_error!(InvalidToken)))
                    }
                } else {
                    Outcome::Error((Status::Forbidden, create_error!(InvalidToken)))
                }
            }
            Outcome::Forward(f) => Outcome::Forward(f),
            Outcome::Error(err) => Outcome::Error(err),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UnvalidatedTicket {
    type Error = Error;

    #[allow(clippy::collapsible_match)]
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match request.guard::<MFATicket>().await {
            Outcome::Success(ticket) => {
                if !ticket.validated {
                    Outcome::Success(UnvalidatedTicket(ticket))
                } else {
                    Outcome::Error((Status::Forbidden, create_error!(InvalidToken)))
                }
            }
            Outcome::Forward(f) => Outcome::Forward(f),
            Outcome::Error(err) => Outcome::Error(err),
        }
    }
}
