use revolt_database::util::reference::Reference;
use revolt_database::{Database, User};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;

/// # Unblock User
///
/// Unblock another user by their id.
#[openapi(tag = "Relationships")]
#[delete("/<target>/block")]
pub async fn unblock(
    db: &State<Database>,
    mut user: User,
    target: Reference<'_>,
) -> Result<Json<v0::User>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let mut target = target.as_user(db).await?;

    user.unblock_user(db, &mut target).await?;
    Ok(Json(target.into(db, &user).await))
}
