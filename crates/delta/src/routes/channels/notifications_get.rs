use revolt_database::{Database, NotificationSetting, User, util::reference::Reference};
use revolt_models::v0::NotificationSettingPreference;
use revolt_result::Result;
use rocket::{State, serde::json::Json};

/// # Get notification preferences for the channel
#[openapi(tag = "Messaging")]
#[get("/<target>/notifications")]
pub async fn notifications_get(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
) -> Result<Json<NotificationSettingPreference>> {
    let channel = target.as_channel(db).await?;

    let pref = NotificationSetting::get_preference_for_channel(
        db,
        &user.id,
        channel.server(),
        channel.id(),
    )
    .await?;

    Ok(Json(pref.into()))
}
