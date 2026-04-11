use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, EmojiParent, PartialEmoji, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

/// # Edit Emoji
///
/// Edit an emoji by its id.
#[openapi(tag = "Emojis")]
#[patch("/emoji/<emoji_id>", data = "<data>")]
pub async fn edit_emoji(
    db: &State<Database>,
    user: User,
    emoji_id: Reference<'_>,
    data: Json<v0::DataEditEmoji>,
) -> Result<Json<v0::Emoji>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let mut emoji = emoji_id.as_emoji(db).await?;

    if emoji.creator_id != user.id {
        match &emoji.parent {
            EmojiParent::Server { id } => {
                let server = db.fetch_server(id.as_str()).await?;

                let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
                calculate_server_permissions(&mut query)
                    .await
                    .throw_if_lacking_channel_permission(ChannelPermission::ManageCustomisation)?;
            }
            EmojiParent::Detached => return Ok(Json(emoji.into())),
        }
    }

    if data.name.is_none() {
        return Ok(Json(emoji.into()));
    }

    let partial = PartialEmoji { name: data.name };
    emoji.update(db, partial).await?;

    Ok(Json(emoji.into()))
}

#[cfg(test)]
mod test {
    use crate::util::test::TestHarness;
    use revolt_database::{Emoji, EmojiParent};
    use revolt_models::v0;
    use rocket::http::{ContentType, Header, Status};
    use ulid::Ulid;

    #[rocket::async_test]
    async fn edit_emoji_name_as_creator() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let emoji_id = Ulid::new().to_string();
        let emoji = Emoji {
            id: emoji_id.clone(),
            parent: EmojiParent::Server {
                id: server.id.clone(),
            },
            creator_id: user.id.clone(),
            name: "initial_name".to_string(),
            animated: false,
            nsfw: false,
        };
        emoji.create(&harness.db).await.expect("`Emoji` created");

        let response = harness
            .client
            .patch(format!("/custom/emoji/{emoji_id}"))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataEditEmoji {
                    name: Some("renamed_emoji".to_string()),
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let edited: v0::Emoji = response.into_json().await.expect("`Emoji`");
        assert_eq!(edited.name, "renamed_emoji");
    }

    #[rocket::async_test]
    async fn reject_invalid_emoji_name() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let emoji_id = Ulid::new().to_string();
        let emoji = Emoji {
            id: emoji_id.clone(),
            parent: EmojiParent::Server {
                id: server.id.clone(),
            },
            creator_id: user.id.clone(),
            name: "valid_name".to_string(),
            animated: false,
            nsfw: false,
        };
        emoji.create(&harness.db).await.expect("`Emoji` created");

        let response = harness
            .client
            .patch(format!("/custom/emoji/{emoji_id}"))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataEditEmoji {
                    name: Some("Invalid Name".to_string()),
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }
}
