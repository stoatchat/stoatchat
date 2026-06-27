use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference}, voice::{sync_voice_permissions, VoiceClient}, Database, User
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Edits server roles ranks
///
/// Edit's server role's ranks.
#[openapi(tag = "Server Permissions")]
#[patch("/<target>/roles/ranks", data = "<data>")]
pub async fn edit_role_ranks(
    db: &State<Database>,
    voice_client: &State<VoiceClient>,
    user: User,
    target: Reference<'_>,
    data: Json<v0::DataEditRoleRanks>,
) -> Result<Json<v0::Server>> {
    let data = data.into_inner();

    let mut server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    calculate_server_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageRole)?;

    let existing_order = server
        .ordered_roles()
        .into_iter()
        .map(|(id, _)| id)
        .collect::<Vec<_>>();

    let new_order = data.ranks.clone().into_iter().collect::<Vec<_>>();

    // Verify all roles are in the new ordering
    if data.ranks.len() != server.roles.len()
        || !server.roles.iter().all(|(id, _)| data.ranks.contains(id))
    {
        return Err(create_error!(InvalidOperation));
    }

    // Don't have to check what the user can't modify if they are the server owner
    if server.owner != user.id {
        let member_top_rank = query.get_member_rank();

        if server
            .roles
            .iter()
            // Find all roles above the member which we should not be able to reorder
            .filter(|(_, role)| {
                if let Some(top_rank) = member_top_rank {
                    role.rank <= top_rank
                } else {
                    true
                }
            })
            // Check if user is trying to reorder roles they can't reorder (as found previously)
            .any(|(id, _)| {
                existing_order
                    .iter()
                    .position(|existing_id| id == existing_id)
                    != new_order.iter().position(|new_id| id == new_id)
            })
        {
            return Err(create_error!(NotElevated));
        }
    }

    server.set_role_ordering(db, new_order).await?;

    for channel_id in &server.channels {
        let channel = Reference::from_unchecked(channel_id).as_channel(db).await?;

        sync_voice_permissions(db, voice_client, &channel, Some(&server), None).await?;
    };

    Ok(Json(server.into()))
}

#[cfg(test)]
mod test {
    use revolt_database::fixture;
    use revolt_models::v0;
    use rocket::http::{ContentType, Header, Status};

    use crate::util::test::TestHarness;

    #[rocket::async_test]
    async fn edit_role_rankings() {
        let harness = TestHarness::new().await;

        fixture!(harness.db, "server_with_many_roles",
            owner user 0
            moderator user 1
            server server 4);

        // Moderator can re-order the roles below them
        let (_, moderator_session) = harness.account_from_user(moderator.id).await;
        let mut target_order: Vec<String> = server
            .ordered_roles()
            .into_iter()
            .map(|(id, _)| id)
            .collect();

        // Swap the two lower ranked roles
        target_order.swap(2, 3);

        let response = harness
            .client
            .patch(format!("/servers/{}/roles/ranks", server.id))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataEditRoleRanks {
                    ranks: target_order.clone()
                })
                .to_string(),
            )
            .header(Header::new(
                "x-session-token",
                moderator_session.token.to_string(),
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        drop(response);

        // ... but not above them
        let mut target_order: Vec<String> = server
            .ordered_roles()
            .into_iter()
            .map(|(id, _)| id)
            .collect();

        // Swap the two lower ranked roles
        target_order.swap(0, 1);

        let response = harness
            .client
            .patch(format!("/servers/{}/roles/ranks", server.id))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataEditRoleRanks {
                    ranks: target_order.clone()
                })
                .to_string(),
            )
            .header(Header::new(
                "x-session-token",
                moderator_session.token.to_string(),
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
        drop(response);

        // The owner can set any order they want
        let (_, owner_session) = harness.account_from_user(owner.id).await;

        let response = harness
            .client
            .patch(format!("/servers/{}/roles/ranks", server.id))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataEditRoleRanks {
                    ranks: target_order.clone()
                })
                .to_string(),
            )
            .header(Header::new(
                "x-session-token",
                owner_session.token.to_string(),
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        drop(response);
    }
}
