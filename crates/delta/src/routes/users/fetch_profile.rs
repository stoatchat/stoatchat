use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_user_permissions, UserPermission};
use revolt_result::Result;
use rocket::{serde::json::Json, State};

/// # Fetch User Profile
///
/// Retrieve a user's profile data.
///
/// Will fail if you do not have permission to access the other user's profile.
#[openapi(tag = "User Information")]
#[get("/<target>/profile")]
pub async fn profile(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
) -> Result<Json<v0::UserProfile>> {
    if user.id == target.id {
        return Ok(Json(user.profile.map(Into::into).unwrap_or_default()));
    }

    let target = target.as_user(db).await?;

    let mut query = DatabasePermissionQuery::new(db, &user).user(&target);
    calculate_user_permissions(&mut query)
        .await
        .throw_if_lacking_user_permission(UserPermission::ViewProfile)?;

    Ok(Json(target.profile.map(Into::into).unwrap_or_default()))
}
