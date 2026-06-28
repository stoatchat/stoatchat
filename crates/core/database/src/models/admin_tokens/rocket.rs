use iso8601_timestamp::Timestamp;
use revolt_config::{capture_internal_error, report_error};
use revolt_result::{Error, Result};
use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};

use crate::{AdminAuthorization, AdminMachineToken, AdminUser, Database};

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminMachineToken {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let user: &Option<AdminMachineToken> = request
            .local_cache_async(async {
                let db = request.rocket().state::<Database>().expect("`Database`");

                if let Some(on_behalf_of) = request
                    .headers()
                    .get("x-admin-on-behalf-of")
                    .next()
                    .map(|x| x.to_string())
                {
                    if let Some(token) = request
                        .headers()
                        .get("x-admin-machine")
                        .next()
                        .map(|x| x.to_string())
                    {
                        let config = revolt_config::config().await;
                        let token = token.to_string();
                        if config.api.security.admin_keys.contains(&token) {
                            let resp: Result<AdminMachineToken> = if on_behalf_of.contains("@") {
                                AdminMachineToken::new_from_email(&on_behalf_of, db).await
                            } else {
                                AdminMachineToken::new_from_id(&on_behalf_of, db).await
                            };
                            if let Ok(resp) = resp {
                                return Some(resp);
                            }
                        }
                    }
                }
                None
            })
            .await;

        if let Some(user) = user {
            Outcome::Success(user.clone())
        } else {
            Outcome::Error((Status::Unauthorized, create_error!(InvalidSession)))
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminAuthorization {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let machine: &Option<AdminMachineToken> = request
            .local_cache_async(async {
                let db = request.rocket().state::<Database>().expect("`Database`");

                if let Some(token) = request
                    .headers()
                    .get("x-admin-machine")
                    .next()
                    .map(|x| x.to_string())
                {
                    if let Some(on_behalf_of) = request
                        .headers()
                        .get("x-admin-on-behalf-of")
                        .next()
                        .map(|x| x.to_string())
                    {
                        let config = revolt_config::config().await;
                        let token = token.to_string();
                        if config.api.security.admin_keys.contains(&token) {
                            let resp: Result<AdminMachineToken> = if on_behalf_of.contains("@") {
                                AdminMachineToken::new_from_email(&on_behalf_of, db).await
                            } else {
                                AdminMachineToken::new_from_id(&on_behalf_of, db).await
                            };
                            if let Ok(resp) = resp {
                                return Some(resp);
                            }
                        }
                    }
                }
                None
            })
            .await;

        if let Some(machine) = machine {
            if !machine.on_behalf_of.active {
                Outcome::Error((Status::Unauthorized, create_error!(LockedOut)))
            } else {
                Outcome::Success(AdminAuthorization::AdminMachine(machine.clone()))
            }
        } else {
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
                            if let Some(expiry) = Timestamp::parse(&admin_token.expiry) {
                                if expiry < Timestamp::now_utc() {
                                    if let Err(e) = db.admin_token_revoke(&admin_token.id).await {
                                        capture_internal_error!(e);
                                    }
                                    return None;
                                } else if let Ok(user) =
                                    db.admin_user_fetch(&admin_token.user_id).await
                                {
                                    if !user.active {
                                        return None;
                                    }
                                    return Some(user);
                                }
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
                    Outcome::Success(AdminAuthorization::AdminUser(user.clone()))
                }
            } else {
                Outcome::Error((Status::Unauthorized, create_error!(InvalidSession)))
            }
        }
    }
}
