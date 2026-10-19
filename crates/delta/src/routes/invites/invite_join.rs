use revolt_database::{util::reference::Reference, Channel, Database, Invite, Member, User, AMQP};
use revolt_models::v0::{self, InviteJoinResponse};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Join Invite
///
/// Join an invite by its ID
#[openapi(tag = "Invites")]
#[post("/<target>")]
pub async fn join(
    db: &State<Database>,
    amqp: &State<AMQP>,
    user: User,
    target: Reference<'_>,
) -> Result<Json<v0::InviteJoinResponse>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    user.can_acquire_server(db).await?;

    let invite = target.as_invite(db).await?;

    // Joining a discoverable server by its ID uses an unstored invite, so there's no use to consume
    let is_discoverable_server =
        matches!(&invite, Invite::Server { code, server, .. } if code == server);

    if !is_discoverable_server && db.consume_invite_use(invite.code()).await?.is_none() {
        return Err(create_error!(NotFound));
    }

    match &invite {
        Invite::Server { server, .. } => {
            let server = db.fetch_server(server).await?;
            let (_, channels) = Member::create(db, &server, &user, None).await?;

            Ok(Json(InviteJoinResponse::Server {
                channels: channels.into_iter().map(|c| c.into()).collect(),
                server: server.into(db).await,
            }))
        }
        Invite::Group {
            channel, creator, ..
        } => {
            let mut channel = db.fetch_channel(channel).await?;
            channel.add_user_to_group(db, amqp, &user, creator).await?;
            if let Channel::Group { recipients, .. } = &channel {
                Ok(Json(InviteJoinResponse::Group {
                    users: User::fetch_many_ids_as_mutuals(db, &user, recipients).await?,
                    channel: channel.into(),
                }))
            } else {
                unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::PartialServer;
    use revolt_models::v0::InviteJoinResponse;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn success_join_discoverable_server_by_id() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (_, session, _) = harness.new_user().await;
        let (server, _) = harness.new_server(&owner).await;

        harness
            .db
            .update_server(
                &server.id,
                &PartialServer {
                    discoverable: Some(true),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to make server discoverable");

        let response = TestHarness::with_session(
            session,
            harness.client.post(format!("/invites/{}", server.id)),
        )
        .await;
        assert_eq!(response.status(), Status::Ok);
        let join: InviteJoinResponse = response.into_json().await.expect("`InviteJoinResponse`");
        match join {
            InviteJoinResponse::Server { server: joined, .. } => assert_eq!(joined.id, server.id),
            _ => unreachable!(),
        }
    }
}
