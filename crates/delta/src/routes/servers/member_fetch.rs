use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, User,
};
use revolt_models::v0;
use revolt_permissions::PermissionQuery;
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// Fetch Member
///
/// Retrieve a member.
#[utoipa::path(
    tag = "Server Members",
    security(("Session-Token" = []), ("Bot-Token" = [])),
    params(
        ("target" = Reference, Path),
        ("member" = Reference, Path),
        (
            "roles" = bool,
            Query,
            description = "Whether to include roles in the response",
            example = json!(true)
        )
    ),
    responses(
        (status = 200, body = v0::MemberResponse),
    ),
)]
#[get("/<target>/members/<member>?<roles>")]
pub async fn fetch(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    member: Reference<'_>,
    roles: Option<bool>,
) -> Result<Json<v0::MemberResponse>> {
    let server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    if !query.are_we_a_member().await {
        return Err(create_error!(NotFound));
    }

    let member = member.as_member(db, &server.id).await?;
    if let Some(true) = roles {
        Ok(Json(v0::MemberResponse::MemberWithRoles {
            roles: server
                .roles
                .into_iter()
                .filter(|(k, _)| member.roles.contains(k))
                .map(|(k, v)| (k, v.into()))
                .collect(),
            member: member.into(),
        }))
    } else {
        Ok(Json(v0::MemberResponse::Member(member.into())))
    }
}
