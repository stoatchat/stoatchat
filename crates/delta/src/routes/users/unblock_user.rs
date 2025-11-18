use revolt_database::util::reference::Reference;
use revolt_database::{Database, User};
use revolt_models::v0;
use revolt_result::Result;
use rocket::serde::json::Json;
use rocket::State;

/// Unblock User
///
/// Unblock another user by their id.
#[utoipa::path(
    tag = "Relationships",
    security(("Session-Token" = []), ("Bot-Token" = [])),
    params(
        ("target" = Reference, Path),
    ),
    responses(
        (status = 200, body = v0::User),
    ),
)]
#[delete("/<target>/block")]
pub async fn unblock(
    db: &State<Database>,
    mut user: User,
    target: Reference<'_>,
) -> Result<Json<v0::User>> {
    let mut target = target.as_user(db).await?;

    user.unblock_user(db, &mut target).await?;
    Ok(Json(target.into(db, &user).await))
}
