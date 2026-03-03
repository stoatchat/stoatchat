//! Fetch your account
//! GET /account
use revolt_database::Account;
use rocket::serde::json::Json;
use revolt_models::v0;
use revolt_result::Result;

/// # Fetch Account
///
/// Fetch account information from the current session.
#[openapi(tag = "Account")]
#[get("/")]
pub async fn fetch_account(account: Account) -> Result<Json<v0::AccountInfo>> {
    Ok(Json(account.into()))
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let res = harness.client
            .get("/auth/account")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert_eq!(
            &res.into_json::<v0::AccountInfo>().await.unwrap().id,
            &account.id
        );
    }
}
