use revolt_database::{util::reference::Reference, Database, User};
use revolt_models::v0::PublicBot;
use revolt_result::{create_error, Result};

use rocket::serde::json::Json;
use rocket::State;

/// # Fetch Public Bot
///
/// Fetch details of a public (or owned) bot by its id.
#[openapi(tag = "Bots")]
#[get("/<target>/invite")]
pub async fn fetch_public_bot(
    db: &State<Database>,
    user: Option<User>,
    target: Reference<'_>,
) -> Result<Json<PublicBot>> {
    let bot = db.fetch_bot(target.id).await?;
    if !bot.public && user.is_none_or(|x| x.id != bot.owner) {
        return Err(create_error!(NotFound));
    }

    let user = db.fetch_user(&bot.id).await?;
    Ok(Json(bot.into_public_bot(user)))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{Bot, PartialBot};
    use revolt_models::v0;

    #[rocket::async_test]
    async fn fetch_public() {
        let harness = TestHarness::new().await;
        let (_, _, user) = harness.new_user().await;

        let (mut bot, _) = Bot::create(&harness.db, TestHarness::rand_string(), &user, None)
            .await
            .expect("`Bot`");

        bot.update(
            &harness.db,
            PartialBot {
                public: Some(true),
                ..Default::default()
            },
            vec![],
        )
        .await
        .unwrap();

        let bot_user = harness.db.fetch_user(&bot.id).await.expect("`User`");
        let response = harness
            .client
            .get(format!("/bots/{}/invite", bot.id))
            .dispatch()
            .await;

        let public_bot: v0::PublicBot = response.into_json().await.expect("`PublicBot`");
        assert_eq!(public_bot, bot.into_public_bot(bot_user));
    }
}
