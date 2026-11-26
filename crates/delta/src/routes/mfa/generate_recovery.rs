//! Re-generate recovery codes for an account.
//! PATCH /mfa/recovery
use revolt_database::{AMQP, Account, Database, MFAMethod, User, ValidatedTicket};
use revolt_result::Result;
use rocket::State;
use rocket::serde::json::Json;

/// # Generate Recovery Codes
///
/// Re-generate recovery codes for an account.
#[openapi(tag = "MFA")]
#[patch("/recovery")]
pub async fn generate_recovery(
    db: &State<Database>,
    amqp: &State<AMQP>,
    mut account: Account,
    user: User,
    _ticket: ValidatedTicket,
) -> Result<Json<Vec<String>>> {
    // Generate new codes
    account.mfa.generate_recovery_codes();

    // Save account model
    account.save(db).await?;

    _ = user
        .send_system_notice(
            db,
            amqp,
            revolt_database::SystemNotice::MfaUpdated {
                add: Some(vec![MFAMethod::Recovery]),
                remove: None,
            },
        )
        .await;

    // Return them to the user
    Ok(Json(account.mfa.recovery_codes))
}

#[cfg(test)]
mod tests {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::MFATicket;
    use rocket::http::{ContentType, Header, Status};
    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let ticket1 = MFATicket::new(account.id.to_string(), true);
        ticket1.save(&harness.db).await.unwrap();

        let ticket2 = MFATicket::new(account.id, true);
        ticket2.save(&harness.db).await.unwrap();

        let res = harness
            .client
            .patch("/auth/mfa/recovery")
            .header(Header::new("X-Session-Token", session.token.clone()))
            .header(Header::new("X-MFA-Ticket", ticket1.token))
            .header(ContentType::JSON)
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert!(res.into_json::<Vec<String>>().await.is_some());

        let res = harness
            .client
            .post("/auth/mfa/recovery")
            .header(Header::new("X-Session-Token", session.token))
            .header(Header::new("X-MFA-Ticket", ticket2.token))
            .header(ContentType::JSON)
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.into_json::<Vec<String>>().await.unwrap().len(), 10);
    }
}
