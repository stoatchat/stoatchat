use revolt_database::{util::reference::Reference, voice::{
    delete_voice_channel, get_user_voice_channel_in_server, remove_user_from_voice_channel,
    UserVoiceChannel, VoiceClient,
}, Database, RemovalIntention, User, ValidatedTicket, Account};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::State;

use rocket_empty::EmptyResponse;

/// # Delete / Leave Server
///
/// Deletes a server if owner otherwise leaves.
#[openapi(tag = "Server Information")]
#[delete("/<target>?<options..>")]
pub async fn delete(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    target: Reference<'_>,
    options: v0::OptionsServerDelete,
    account: Account,
    validated_ticket: Option<ValidatedTicket>,
) -> Result<EmptyResponse> {
    let server = target.as_server(db).await?;
    let member = db.fetch_member(target.id, &user.id).await?;

    if server.owner == user.id {
        if account.mfa.is_active() && validated_ticket.is_none() {
            return Err(create_error!(InvalidCredentials));
        }

        for channel_id in &server.channels {
            delete_voice_channel(
                voice_client,
                &UserVoiceChannel {
                    id: channel_id.clone(),
                    server_id: Some(server.id.clone()),
                },
            )
            .await?;
        }

        server.delete(db).await
    } else {
        if let Some(channel_id) = get_user_voice_channel_in_server(&user.id, &server.id).await? {
            remove_user_from_voice_channel(
                voice_client,
                &UserVoiceChannel {
                    id: channel_id,
                    server_id: Some(server.id.clone()),
                },
                &user.id,
            )
            .await?;
        };

        member
            .remove(
                db,
                &server,
                RemovalIntention::Leave,
                options.leave_silently.unwrap_or_default(),
            )
            .await
    }
    .map(|_| EmptyResponse)
}


#[cfg(test)]
mod test {
    use rocket::http::Header;
    use revolt_database::{MFATicket, Member, Server, Totp};
    use revolt_database::events::client::EventV1;
    use revolt_models::v0::DataCreateServer;
    use crate::routes::Status;
    use crate::util::test::{PubSubTestHelper, TestHarness};

    #[rocket::async_test]
    async fn delete_server() {
        let harness = TestHarness::new().await;
        let (mut account, session, user) = harness.new_user().await;

        let (server, channels) = Server::create(
            &harness.db,
            DataCreateServer {
                name: TestHarness::rand_string(),
                ..Default::default()
            },
            &user,
            false,
        )
        .await
        .unwrap();
        let mut pubsub = PubSubTestHelper::new(&server.id).await;
        Member::create(&harness.db, &server, &user, channels.into(),)
            .await
            .unwrap();

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.email = "delete_server@smtp.test".to_string();
        account.save(&harness.db).await.unwrap();

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let response = harness
            .client
            .delete(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NoContent);
        assert!(harness.db.fetch_server(&server.id).await.is_err());
        drop(response);

        pubsub
            .wait_for_event(|event| match event {
                EventV1::ServerDelete { id, .. } => id == &server.id,
                _ => false,
            })
            .await;
    }

    #[rocket::async_test]
    async fn mfa_fail() {
        let harness = TestHarness::new().await;
        let (mut account, session, user) = harness.new_user().await;

        let (server, channels) = Server::create(
            &harness.db,
            DataCreateServer {
                name: TestHarness::rand_string(),
                ..Default::default()
            },
            &user,
            false,
        )
            .await
            .unwrap();
        Member::create(&harness.db, &server, &user, channels.into(),)
            .await
            .unwrap();

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.email = "delete_server@smtp.test".to_string();
        account.save(&harness.db).await.unwrap();

        let response = harness
            .client
            .delete(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Unauthorized);
    }
}