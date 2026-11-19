use revolt_database::{Database, NotificationSetting, User, util::reference::Reference};
use revolt_models::v0::NotificationSettingPreference;
use revolt_result::Result;
use rocket::{State, serde::json::Json};

/// # Get notification preferences for the server
///
/// Fetch the kind of notifications you receive for this server. Channels may override this preference
#[openapi(tag = "Server Information")]
#[get("/<target>/notifications")]
pub async fn notifications_get(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
) -> Result<Json<NotificationSettingPreference>> {
    let server = target.as_server(db).await?;

    let pref = NotificationSetting::get_preference_for_server(db, &user.id, &server.id).await?;

    Ok(Json(pref.into()))
}
