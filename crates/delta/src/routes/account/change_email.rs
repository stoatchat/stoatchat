//! Change account email.
//! PATCH /account/change/email
use revolt_database::util::email::validate_email;
use revolt_database::{Database, Account};
use revolt_models::v0;
use rocket::serde::json::Json;
use rocket::State;
use rocket_empty::EmptyResponse;
use revolt_result::Result;

/// # Change Email
///
/// Change the associated account email.
#[openapi(tag = "Account")]
#[patch("/change/email", data = "<data>")]
pub async fn change_email(
    db: &State<Database>,
    mut account: Account,
    data: Json<v0::DataChangeEmail>,
) -> Result<EmptyResponse> {
    let data = data.into_inner();

    validate_email(&data.email)?;

    // Ensure given password is correct
    account.verify_password(&data.current_password)?;

    // Send email verification for new email
    account
        .start_email_move(db, data.email)
        .await
        .map(|_| EmptyResponse)
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_models::v0;
    use rocket::http::{ContentType, Header, Status};

    // #[async_std::test]
    // async fn success() {
    //     let harness = TestHarness::new().await;
    //     let (account, session, _) = harness.new_user().await;

    //     let res = harness.client
    //         .patch("/auth/account/change/email")
    //         .header(ContentType::JSON)
    //         .header(Header::new("X-Session-Token", session.token.clone()))
    //         .body(
    //             json!({
    //                 "email": "validexample@valid.com",
    //                 "current_password": "password_insecure"
    //             })
    //             .to_string(),
    //         )
    //         .dispatch()
    //         .await;

    //     assert_eq!(res.status(), Status::NoContent);

    //     let account = harness.db.fetch_account(&account.id).await.unwrap();

    //     assert_eq!(account.email, "validexample@valid.com");
    // }

    #[async_std::test]
    async fn success_smtp() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let res = harness.client
            .patch("/auth/account/change/email")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token.clone()))
            .body(
                json!({
                    "email": "change_email@smtp.test",
                    "current_password": "password_insecure"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NoContent);

        let account = harness.db.fetch_account(&account.id).await.unwrap();
        println!("{account:?}");

        let (_, code) = harness.assert_email("change_email@smtp.test").await;
        let res = harness.client
            .post(format!("/auth/account/verify/{}", code))
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);

        let account = harness.db.fetch_account(&account.id).await.unwrap();

        assert_eq!(account.email, "change_email@smtp.test");

        // Ensure that we did not receive a ticket
        assert_eq!(
            v0::ResponseVerify::NoTicket,
            res.into_json().await.expect("`ResponseVerify")
        )
    }
}
