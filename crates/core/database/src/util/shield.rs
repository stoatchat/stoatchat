use reqwest::Client;
use revolt_config::config;
use revolt_result::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::LazyLock};
use crate::util::ip;

static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ShieldValidationInput {
    /// Remote user IP
    pub ip: Option<String>,

    /// User provided email
    pub email: Option<String>,

    /// Request headers
    pub headers: Option<HashMap<String, String>>,

    /// Skip alerts and monitoring for this request
    pub dry_run: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether this request was blocked
    blocked: bool,

    /// Reasons for the request being blocked
    reasons: Vec<String>,
}

pub async fn validate_shield(input: ShieldValidationInput) -> Result<()> {
    let shield = config().await.api.security.shield;

    if !shield.host.is_empty() {
        if let Ok(response) = CLIENT
            .post(format!("{}/validate", &shield.host))
            .json(&input)
            .header("Authorization", &shield.key)
            .send()
            .await
        {
            let result = response
                .json::<ValidationResult>()
                .await
                .map_err(|_| create_error!(InternalError))?;

            if result.blocked {
                return Err(create_error!(BlockedByShield));
            }
        }
    }

    Ok(())
}

#[cfg(feature = "rocket-impl")]
#[async_trait]
impl<'r> rocket::request::FromRequest<'r> for ShieldValidationInput {
    type Error = revolt_result::Error;

    #[allow(clippy::collapsible_match)]
    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        rocket::request::Outcome::Success(ShieldValidationInput {
            ip: Some(ip::rocket::to_real_ip(request).await),
            headers: Some(
                request
                    .headers()
                    .iter()
                    .map(|entry| (entry.name.to_string(), entry.value.to_string()))
                    .collect(),
            ),
            ..Default::default()
        })
    }
}

#[cfg(feature = "rocket-impl")]
impl<'r> revolt_rocket_okapi::request::OpenApiFromRequest<'r> for ShieldValidationInput {
    fn from_request_input(
        _gen: &mut revolt_rocket_okapi::r#gen::OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<revolt_rocket_okapi::request::RequestHeaderInput> {
        Ok(revolt_rocket_okapi::request::RequestHeaderInput::None)
    }
}
#[cfg(feature = "axum-impl")]
#[async_trait]
impl<S> axum::extract::FromRequestParts<S> for ShieldValidationInput {
    type Rejection = axum::Json<revolt_result::Error> ;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(ShieldValidationInput {
            ip: Some(ip::axum::to_real_ip(parts).await),
            headers: Some(
                parts
                    .headers
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.to_string(),
                            value.to_str().map(|s| s.to_string()).unwrap_or_default(),
                        )
                    })
                    .collect(),
            ),
            ..Default::default()
        })
    }
}
