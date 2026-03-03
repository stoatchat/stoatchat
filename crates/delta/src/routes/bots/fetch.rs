use revolt_database::{util::reference::Reference, Database, User};
use revolt_models::v0::FetchBotResponse;
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Fetch Bot
///
/// Fetch details of a bot you own by its id.
#[openapi(tag = "Bots")]
#[get("/<bot_id>")]
pub async fn fetch_bot(
    db: &State<Database>,
    user: User,
    bot_id: Reference<'_>,
) -> Result<Json<FetchBotResponse>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let bot = bot_id.as_bot(db).await?;
    if bot.owner != user.id {
        return Err(create_error!(NotFound));
    }

    Ok(Json(FetchBotResponse {
        user: db.fetch_user(&bot.id).await?.into(db, None).await,
        bot: bot.into(),
    }))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::Bot;
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    #[rocket::async_test]
    async fn fetch_bot() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let (bot, _) = Bot::create(&harness.db, TestHarness::rand_string(), &user, None)
            .await
            .expect("`Bot`");

        let response = harness
            .client
            .get(format!("/bots/{}", bot.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let response: v0::FetchBotResponse = response.into_json().await.expect("`Bot`");
        assert_eq!(response.bot, bot.into());
    }
}
