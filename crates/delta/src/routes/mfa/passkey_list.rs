//! Fetch registered passkeys.
//! GET /mfa/passkeys
use revolt_database::Account;
use revolt_models::v0;
use rocket::serde::json::Json;

/// # List Passkeys
///
/// Fetch passkeys registered to this account
#[openapi(tag = "MFA")]
#[get("/passkeys")]
pub async fn passkey_list(account: Account) -> Json<Vec<v0::PasskeyInfo>> {
    Json(account.mfa.passkeys.into_iter().map(|p| p.into()).collect())
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness
            .client
            .get("/auth/mfa/passkeys")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(
            res.into_json::<Vec<v0::PasskeyInfo>>()
                .await
                .unwrap()
                .is_empty()
        );
    }
}
