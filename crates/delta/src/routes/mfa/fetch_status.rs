//! Fetch MFA status of an account.
//! GET /mfa
use revolt_database::Account;
use revolt_result::Result;
use revolt_models::v0;
use rocket::serde::json::Json;

/// # MFA Status
///
/// Fetch MFA status of an account.
#[openapi(tag = "MFA")]
#[get("/")]
pub async fn fetch_status(account: Account) -> Result<Json<v0::MultiFactorStatus>> {
    Ok(Json(account.mfa.into()))
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use rocket::http::{Header, Status};
    use revolt_models::v0;

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness.client
            .get("/auth/mfa")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        println!("{:?}", res.into_string().await);

        // assert_eq!(res.status(), Status::Ok);
        // assert!(res.into_json::<v0::MultiFactorStatus>().await.is_some());
    }
}
