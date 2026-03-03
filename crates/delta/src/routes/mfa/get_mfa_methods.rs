//! Fetch available MFA methods.
//! GET /mfa/methods
use revolt_database::Account;
use revolt_models::v0;
use rocket::serde::json::Json;

/// # Get MFA Methods
///
/// Fetch available MFA methods.
#[openapi(tag = "MFA")]
#[get("/methods")]
pub async fn get_mfa_methods(account: Account) -> Json<Vec<v0::MFAMethod>> {
    Json(
        account
            .mfa
            .get_methods()
            .into_iter()
            .map(Into::into)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::Totp;
    use rocket::http::{Header, Status};
    use revolt_models::v0;

    #[async_std::test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness.client
            .get("/auth/mfa/methods")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert_eq!(
            res.into_json::<Vec<v0::MFAMethod>>().await.unwrap(),
            vec![v0::MFAMethod::Password]
        );
    }

    #[async_std::test]
    async fn success_has_recovery_and_totp() {
        let harness = TestHarness::new().await;
        let (mut account, session, _) = harness.new_user().await;

        account.mfa.totp_token = Totp::Enabled {
            secret: "some".to_string(),
        };
        account.mfa.generate_recovery_codes();
        account.save(&harness.db).await.unwrap();

        let res = harness.client
            .get("/auth/mfa/methods")
            .header(Header::new("X-Session-Token", session.token))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert_eq!(
            res.into_json::<Vec<v0::MFAMethod>>().await.unwrap(),
            vec![v0::MFAMethod::Totp, v0::MFAMethod::Recovery]
        );
    }
}
