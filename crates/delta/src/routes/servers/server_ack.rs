use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, User,
};
use revolt_permissions::PermissionQuery;
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// Mark Server As Read
///
/// Mark all channels in a server as read.
#[utoipa::path(
    tag = "Server Permissions",
    security(("Session-Token" = [])),
    params(
        ("target" = Reference, Path),
    ),
    responses(
        (status = 204),
    ),
)]
#[put("/<target>/ack")]
pub async fn ack(db: &State<Database>, user: User, target: Reference<'_>) -> Result<EmptyResponse> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    if !query.are_we_a_member().await {
        return Err(create_error!(NotFound));
    }

    db.acknowledge_channels(&user.id, &server.channels)
        .await
        .map(|_| EmptyResponse)
}
