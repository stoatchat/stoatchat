use revolt_database::{Database, Message, MessageFilter, MessageQuery, MessageTimePeriod, User};
use revolt_models::v0::{self, MessageSort};
use revolt_result::Result;
use rocket::{serde::json::Json, State};

#[openapi(tag = "User")]
#[get("/@me/notifications")]
pub async fn notifications(
    db: &State<Database>,
    user: User,
) -> Result<Json<v0::BulkMessageResponse>> {
    let query = MessageQuery {
        filter: MessageFilter {
            mentioned: Some(user.id.to_string()), // Search for the user's ID in mentions
            ..Default::default()
        },
        time_period: MessageTimePeriod::Absolute {
            before: None,
            after: None,
            sort: Some(MessageSort::Latest),
        },
        limit: Some(50),
    };

    Message::fetch_with_users(
        db,
        query,
        &user,
        Some(true),
        None,
    )
        .await
        .map(Json)
}
