use revolt_database::{util::reference::Reference, voice::{remove_user_from_voice_channels, VoiceClient}, Database, User, ValidatedTicket, Account};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Delete Bot
///
/// Delete a bot by its id.
#[openapi(tag = "Bots")]
#[delete("/<bot_id>")]
pub async fn delete_bot(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    bot_id: Reference<'_>,
    account: Account,
    validated_ticket: Option<ValidatedTicket>,
) -> Result<EmptyResponse> {
    let bot = bot_id.as_bot(db).await?;
    if bot.owner != user.id {
        return Err(create_error!(NotFound));
    }
    if account.mfa.is_active() && validated_ticket.is_none() {
        return Err(create_error!(InvalidCredentials));
    }

    bot.delete(db).await?;

    remove_user_from_voice_channels(voice_client, &bot.id).await?;

    Ok(EmptyResponse)
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{events::client::EventV1, Bot, Totp, MFATicket};
    use rocket::http::{Header, Status};
    use crate::util::test::PubSubTestHelper;

    #[rocket::async_test]
    async fn delete_bot() {
        let harness = TestHarness::new().await;
        let (mut account, session, user) = harness.new_user().await;

        let (bot, _) = Bot::create(&harness.db, TestHarness::rand_string(), &user, None)
            .await
            .expect("`Bot`");
        let mut pubsub = PubSubTestHelper::new(&bot.id).await;

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.email = "delete_account@smtp.test".to_string();
        account.save(&harness.db).await.unwrap();

        let ticket = MFATicket::new(account.id.to_string(), true);
        ticket.save(&harness.db).await.unwrap();

        let response = harness
            .client
            .delete(format!("/bots/{}", bot.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(Header::new("X-MFA-Ticket", ticket.token))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NoContent);
        assert!(harness.db.fetch_bot(&bot.id).await.is_err());
        drop(response);

        let event = pubsub
            .wait_for_event(|event| match event {
                EventV1::UserUpdate { id, .. } => id == &bot.id,
                _ => false,
            })
            .await;

        match event {
            EventV1::UserUpdate { data, .. } => {
                assert_eq!(data.flags, Some(2));
            }
            _ => unreachable!(),
        }
    }

    #[rocket::async_test]
    async fn fail_mfa() {
        let harness = TestHarness::new().await;
        let (mut account, session, user) = harness.new_user().await;
        let (bot, _) = Bot::create(&harness.db, TestHarness::rand_string(), &user, None)
            .await
            .expect("`Bot`");

        let totp = Totp::Enabled {
            secret: "secret".to_string(),
        };

        account.mfa.totp_token = totp.clone();
        account.email = "delete_account@smtp.test".to_string();
        account.save(&harness.db).await.unwrap();

        let response = harness
            .client
            .delete(format!("/bots/{}", bot.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Unauthorized);
    }
}
