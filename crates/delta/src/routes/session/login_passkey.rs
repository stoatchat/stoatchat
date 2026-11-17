//! Begin a passwordless login using a passkey.
//! POST /session/login/passkey
use revolt_database::Account;
use revolt_models::v0;
use revolt_result::Result;
use rocket::serde::json::Json;

/// # Begin Passkey Login
///
/// Create a challenge for passwordless login. Send the result to `POST /session/login`
#[openapi(tag = "Session")]
#[post("/login/passkey")]
pub async fn login_passkey() -> Result<Json<v0::ResponsePasskeyChallenge>> {
    let (challenge, options) = Account::start_passkey_login().await?;
    Ok(Json(v0::ResponsePasskeyChallenge { challenge, options }))
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_models::v0;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;

        let res = harness
            .client
            .post("/auth/session/login/passkey")
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        res.into_json::<v0::ResponsePasskeyChallenge>()
            .await
            .unwrap();
    }
}
