use revolt_database::{Database, NotificationSetting, User, util::reference::Reference};
use revolt_models::v0;
use revolt_result::Result;
use rocket::{State, serde::json::Json};
use rocket_empty::EmptyResponse;

/// # Set notification preferences for the channel
///
/// Pass the kind of notifications you want to receive for this channel. This overrides any server preferences
#[openapi(tag = "Messaging")]
#[put("/<target>/notifications", data = "<data>")]
pub async fn notifications_set(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    data: Json<v0::NotificationSettingPreference>,
) -> Result<EmptyResponse> {
    let channel = target.as_channel(db).await?;

    // This endpoint does not check if you have permissions to the channel in question.
    // Setting a preference to a channel you can't access does not mean you'll receive notifications for that channel.
    let setting = NotificationSetting {
        channel: Some(channel.id().to_string()),
        server: channel.server().map(|s| s.to_string()),
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

    async fn get(harness: &TestHarness, token: &str, path: &str) -> NotificationSettingPreference {
        let response = harness
            .client
            .get(path)
            .header(Header::new("x-session-token", token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        response.into_json().await.expect("`NotificationSettingPreference`")
    }

    async fn set(
        harness: &TestHarness,
        token: &str,
        path: &str,
        preference: NotificationSettingPreference,
    ) {
        let response = harness
            .client
            .put(path)
            .header(Header::new("x-session-token", token.to_string()))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&preference).unwrap())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NoContent);
    }

    #[rocket::async_test]
    async fn channel_override() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;
        let channel = &channels[0];
        let other = harness.new_channel(&server).await;

        let channel_path = format!("/channels/{}/notifications", channel.id());
        let other_path = format!("/channels/{}/notifications", other.id());
        let server_path = format!("/servers/{}/notifications", server.id);

        // Defaults to mentions
        assert_eq!(
            get(&harness, &session.token, &channel_path).await,
            NotificationSettingPreference::Mentions
        );

        // Server preference applies to all channels
        set(&harness, &session.token, &server_path, NotificationSettingPreference::None).await;
        assert_eq!(
            get(&harness, &session.token, &channel_path).await,
            NotificationSettingPreference::None
        );

        // Channel override only applies to that channel
        set(
            &harness,
            &session.token,
            &channel_path,
            NotificationSettingPreference::AllMessages,
        )
        .await;
        assert_eq!(
            get(&harness, &session.token, &channel_path).await,
            NotificationSettingPreference::AllMessages
        );
        assert_eq!(
            get(&harness, &session.token, &other_path).await,
            NotificationSettingPreference::None
        );

        // Server preference doesn't replace the channel override
        assert_eq!(
            get(&harness, &session.token, &server_path).await,
            NotificationSettingPreference::None
        );

        // Setting the channel back to mentions removes the override, falling back to the server
        set(
            &harness,
            &session.token,
            &channel_path,
            NotificationSettingPreference::Mentions,
        )
        .await;
        assert_eq!(
            get(&harness, &session.token, &channel_path).await,
            NotificationSettingPreference::None
        );
    }

    #[rocket::async_test]
    async fn settings_are_per_user() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (_, other_session, _) = harness.new_user().await;
        let (_, channels) = harness.new_server(&user).await;

        let path = format!("/channels/{}/notifications", channels[0].id());

        set(&harness, &session.token, &path, NotificationSettingPreference::None).await;
        assert_eq!(
            get(&harness, &other_session.token, &path).await,
            NotificationSettingPreference::Mentions
        );
    }

    #[rocket::async_test]
    async fn unknown_channel() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let response = harness
            .client
            .get("/channels/01ARZ3NDEKTSV4RRFFQ69G5FAV/notifications")
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NotFound);
    }
}
