//! Rename a passkey.
//! PATCH /mfa/passkeys/:id
use revolt_database::{Account, Database};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;
use validator::Validate;

/// # Edit Passkey
///
/// Rename a registered passkey.
#[openapi(tag = "MFA")]
#[patch("/passkeys/<id>", data = "<data>")]
pub async fn passkey_edit(
    db: &State<Database>,
    mut account: Account,
    id: String,
    data: Json<v0::DataEditPasskey>,
) -> Result<Json<v0::PasskeyInfo>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let passkey = account
        .mfa
        .passkeys
        .iter_mut()
        .find(|passkey| passkey.id == id)
        .ok_or_else(|| create_error!(NotFound))?;

    passkey.name = data.name;
    let passkey = passkey.clone();

    account.save(db).await?;
    Ok(Json(passkey.into()))
}

#[cfg(test)]
mod tests {
    use crate::{
        rocket,
        util::{passkey::register_passkey, test::TestHarness},
    };
    use revolt_models::v0;
    use rocket::http::{ContentType, Header, Status};
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;

    #[rocket::async_test]
    async fn success() {
        let harness = TestHarness::new().await;
        let (account, session, _) = harness.new_user().await;

        let mut authenticator = SoftPasskey::new(true);
        let passkey = register_passkey(&harness, &account, &session, &mut authenticator).await;

        let res = harness
            .client
            .patch(format!("/auth/mfa/passkeys/{}", passkey.id))
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token))
            .body(json!({ "name": "Renamed" }).to_string())
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::Ok);
        assert_eq!(
            res.into_json::<v0::PasskeyInfo>().await.unwrap().name,
            "Renamed"
        );
    }

    #[rocket::async_test]
    async fn fail_unknown_passkey() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let res = harness
            .client
            .patch("/auth/mfa/passkeys/unknown")
            .header(ContentType::JSON)
            .header(Header::new("X-Session-Token", session.token))
            .body(json!({ "name": "Renamed" }).to_string())
            .dispatch()
            .await;

        assert_eq!(res.status(), Status::NotFound);
    }
}
