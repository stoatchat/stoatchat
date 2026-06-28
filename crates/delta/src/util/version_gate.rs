//! Rejects requests from clients below the configured `min_client_version`.
//!
//! Mirrors the pattern used by `revolt_ratelimits::rocket::RatelimitFairing`:
//! a `Kind::Request | Kind::Response` fairing checks the condition in
//! `on_request` and, if it fails, redirects the request to a dedicated
//! catch-all route; `on_response` then fixes up the status code and body.
//! This avoids having to add a request guard to every single route.

use std::io::Cursor;

use async_trait::async_trait;
use revolt_config::{client_version_satisfies_minimum, config};
use revolt_result::{create_error, Error};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::uri::Origin;
use rocket::http::{ContentType, Method, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::{Data, Request, Response};

const CLIENT_VERSION_HEADER: &str = "X-Client-Version";
const GATE_ROUTE: &str = "/upgrade-required";

/// Whether the current request satisfies the configured minimum client version.
/// Cached per-request so `on_request` and `on_response` agree on the outcome
/// even though they run as two separate fairing calls.
struct VersionGateOutcome(bool);

#[async_trait]
impl<'r> FromRequest<'r> for &'r VersionGateOutcome {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let outcome = request
            .local_cache_async(async {
                let min_version = config().await.min_client_version.clone();
                let client_version = request.headers().get_one(CLIENT_VERSION_HEADER);
                VersionGateOutcome(client_version_satisfies_minimum(
                    client_version,
                    min_version.as_deref(),
                ))
            })
            .await;

        Outcome::Success(outcome)
    }
}

pub struct VersionGateFairing;

#[async_trait]
impl Fairing for VersionGateFairing {
    fn info(&self) -> Info {
        Info {
            name: "Client Version Gate",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        // Never gate the route that serves the rejection itself.
        if request.uri().path() == GATE_ROUTE {
            return;
        }

        let VersionGateOutcome(allowed) = request.guard::<&VersionGateOutcome>().await.unwrap();
        if !allowed {
            request.set_method(Method::Get);
            request.set_uri(Origin::parse(GATE_ROUTE).unwrap());
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if request.uri().path() != GATE_ROUTE {
            return;
        }

        let min_version = config()
            .await
            .min_client_version
            .clone()
            .unwrap_or_default();

        let error: Error = create_error!(UpgradeRequired { min_version });
        let body = serde_json::to_string(&error).unwrap();

        response.set_status(Status::new(426));
        response.set_header(ContentType::new("application", "json"));
        response.set_sized_body(body.len(), Cursor::new(body));
    }
}

#[rocket::get("/upgrade-required")]
fn upgrade_required() -> &'static str {
    // Body is overwritten by `VersionGateFairing::on_response`; this only
    // exists so Rocket has a route to dispatch to.
    ""
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![upgrade_required]
}
