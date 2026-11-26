use revolt_database::{Database, NotificationSetting, User, util::reference::Reference};
use revolt_models::v0;
use revolt_result::Result;
use rocket::{State, serde::json::Json};
use rocket_empty::EmptyResponse;

/// # Set notification preferences for the server
///
/// Pass the kind of notifications you want to receive for this server. Channels may override this preference
#[openapi(tag = "Server Information")]
#[put("/<target>/notifications", data = "<data>")]
pub async fn notifications_set(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    data: Json<v0::NotificationSettingPreference>,
) -> Result<EmptyResponse> {
    let server = target.as_server(db).await?;

    // This endpoint does not check if you are a member of the server in question.
    // Setting a preference to a server you aren't in does not mean you'll receive notifications for that server.
    let setting = NotificationSetting {
        channel: None,
        server: Some(server.id),
        user: user.id,
        preference: data.0.into(),
    };

    setting.save(db).await?;
    Ok(EmptyResponse)
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_models::v0::NotificationSettingPreference;
    use rocket::http::{ContentType, Header, Status};

    #[rocket::async_test]
    async fn set_and_get() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;
        let path = format!("/servers/{}/notifications", server.id);

        for preference in [
            NotificationSettingPreference::Mentions,
            NotificationSettingPreference::AllMessages,
            NotificationSettingPreference::None,
            NotificationSettingPreference::Mentions,
        ] {
            let response = harness
                .client
                .put(&path)
                .header(Header::new("x-session-token", session.token.to_string()))
                .header(ContentType::JSON)
                .body(serde_json::to_string(&preference).unwrap())
                .dispatch()
                .await;
            assert_eq!(response.status(), Status::NoContent);
            drop(response);

            let response = harness
                .client
                .get(&path)
                .header(Header::new("x-session-token", session.token.to_string()))
                .dispatch()
                .await;
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(
                response
                    .into_json::<NotificationSettingPreference>()
                    .await
                    .unwrap(),
                preference
            );
        }
    }

    #[rocket::async_test]
    async fn unknown_server() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let response = harness
            .client
            .put("/servers/01ARZ3NDEKTSV4RRFFQ69G5FAV/notifications")
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&NotificationSettingPreference::None).unwrap())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NotFound);
    }
}
