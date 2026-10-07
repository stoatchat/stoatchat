use revolt_database::{Database, Notification, User, InboxOptions};
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
    let items = Notification::fetch_inbox(
        db,
        &user,
        InboxOptions {
            limit: options.limit.unwrap_or(50),
            before: options.before,
            mentions: options.mentions.unwrap_or(true),
            roles: options.roles.unwrap_or(true),
            everyone: options.everyone.unwrap_or(true),
        },
    )
    .await?;

    Ok(Json(items.into_iter().map(Into::into).collect()))
}
