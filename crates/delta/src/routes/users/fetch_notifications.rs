use revolt_database::{Database, Notification, User};
use revolt_models::v0;
use revolt_result::Result;
use rocket::{serde::json::Json, State};

/// # Fetch User Notifications
///
/// Retrieve a user's notificationss
#[openapi(tag = "User Information")]
#[get("/notifications?<options..>")]
pub async fn fetch_notifications(
    db: &State<Database>,
    user: User,
    options: v0::OptionsFetchNotifications,
) -> Result<Json<Vec<v0::Notification>>> {
    let notifications = Notification::fetch(
        db,
        &user.id,
        options.before.as_deref(),
        options.limit.unwrap_or(50),
    )
    .await?;

    // TODO: drop rows whose channel the user can no longer view

    Ok(Json(notifications.into_iter().map(Into::into).collect()))
}
