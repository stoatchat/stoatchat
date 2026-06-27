use revolt_database::{util::reference::Reference, Channel, Database, Invite};
use revolt_models::v0;
use revolt_result::Result;
use rocket::{serde::json::Json, State};

/// # Fetch Invite
///
/// Fetch an invite by its id.
#[openapi(tag = "Invites")]
#[get("/<target>")]
pub async fn fetch(db: &State<Database>, target: Reference<'_>) -> Result<Json<v0::InviteResponse>> {
    Ok(Json(match target.as_invite(db).await? {
        Invite::Server {
            channel, creator, ..
        } => {
            let channel = db.fetch_channel(&channel).await?;
            let user = db.fetch_user(&creator).await?;

            match channel {
                Channel::TextChannel {
                    id,
                    server,
                    name,
                    description,
                    ..
                } => {
                    let server = db.fetch_server(&server).await?;

                    v0::InviteResponse::Server {
                        code: target.id.to_string(),
                        member_count: db.fetch_member_count(&server.id).await? as i64,
                        server_id: server.id,
                        server_name: server.name,
                        server_icon: server.icon.map(|f| f.into()),
                        server_banner: server.banner.map(|f| f.into()),
                        server_flags: server.flags,
                        channel_id: id,
                        channel_name: name,
                        channel_description: description,
                        user_name: user.username,
                        user_avatar: user.avatar.map(|f| f.into()),
                    }
                }
                _ => unreachable!(),
            }
        }
        Invite::Group {
            channel, creator, ..
        } => {
            let channel = db.fetch_channel(&channel).await?;
            let user = db.fetch_user(&creator).await?;

            match channel {
                Channel::Group {
                    id,
                    name,
                    description,
                    ..
                } => v0::InviteResponse::Group {
                    code: target.id.to_string(),
                    channel_id: id,
                    channel_name: name,
                    channel_description: description,
                    user_name: user.username,
                    user_avatar: user.avatar.map(|f| f.into()),
                },
                _ => unreachable!(),
            }
        }
    }))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{Channel, Server};
    use revolt_models::v0::{
        DataCreateGroup, DataCreateServerChannel, Invite, InviteResponse, LegacyServerChannelType,
    };
    use rocket::http::Status;

    #[rocket::async_test]
    async fn success_fetch_group_invite() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let group = Channel::create_group(
            &harness.db,
            DataCreateGroup {
                ..Default::default()
            },
            user.id.clone(),
        )
        .await
        .expect("`Channel`");
        let create_response = TestHarness::with_session(
            session,
            harness
                .client
                .post(format!("/channels/{}/invites", group.id())),
        )
        .await;
        assert_eq!(create_response.status(), Status::Ok);
        let invite_from_create: Invite = create_response.into_json().await.expect("`Invite`");
        let invite_code = match invite_from_create {
            Invite::Group { code, .. } => code,
            _ => unreachable!(),
        };
        let response = harness
            .client
            .get(format!("/invites/{}", invite_code))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let invite_response: InviteResponse = response.into_json().await.expect("`FetchInvite`");
        match invite_response {
            InviteResponse::Group {
                code,
                channel_id,
                user_name,
                ..
            } => {
                assert_eq!(code, invite_code);
                assert_eq!(channel_id, group.id());
                assert_eq!(user_name, user.username);
            }
            _ => unreachable!(),
        }
    }

    #[rocket::async_test]
    async fn fail_fetch_missing_invite() {
        let harness = TestHarness::new().await;
        let response = harness
            .client
            .get(format!("/invites/{}", TestHarness::rand_string()))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn success_fetch_text_channel_invite() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (_, channels) = harness.new_server(&user).await;
        let channel = channels.first().expect("Server Channel");
        let create_response = TestHarness::with_session(
            session,
            harness
                .client
                .post(format!("/channels/{}/invites", channel.id())),
        )
        .await;
        assert_eq!(create_response.status(), Status::Ok);
        let invite_from_create: Invite = create_response.into_json().await.expect("`Invite`");
        let invite_code = match invite_from_create {
            Invite::Server { code, .. } => code,
            _ => unreachable!(),
        };
        let response = harness
            .client
            .get(format!("/invites/{}", invite_code))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let invite_response: InviteResponse = response.into_json().await.expect("`FetchInvite`");
        match invite_response {
            InviteResponse::Server {
                code,
                channel_id,
                user_name,
                ..
            } => {
                assert_eq!(code, invite_code);
                assert_eq!(channel_id, channel.id());
                assert_eq!(user_name, user.username);
            }
            _ => unreachable!(),
        };
    }

    #[rocket::async_test]
    async fn success_fetch_voice_channel_invite() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;
        let server_mut: &mut Server = &mut server.clone();

        let channel = Channel::create_server_channel(
            &harness.db,
            server_mut,
            DataCreateServerChannel {
                channel_type: LegacyServerChannelType::Voice,
                name: "Voice Channel".to_string(),
                description: None,
                nsfw: Some(false),
                voice: None
            },
            true,
        )
        .await
        .expect("Failed to make new channel");
        let create_response = TestHarness::with_session(
            session,
            harness
                .client
                .post(format!("/channels/{}/invites", channel.id())),
        )
        .await;
        assert_eq!(create_response.status(), Status::Ok);
        let invite_from_create: Invite = create_response.into_json().await.expect("`Invite`");
        let invite_code = match invite_from_create {
            Invite::Server { code, .. } => code,
            _ => unreachable!(),
        };
        let response = harness
            .client
            .get(format!("/invites/{}", invite_code))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let invite_response: InviteResponse = response.into_json().await.expect("`FetchInvite`");
        match invite_response {
            InviteResponse::Server {
                code,
                channel_id,
                user_name,
                ..
            } => {
                assert_eq!(code, invite_code);
                assert_eq!(channel_id, channel.id());
                assert_eq!(user_name, user.username);
            }
            _ => unreachable!(),
        };
    }
}
