use std::collections::HashSet;

use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, File, PartialServer, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_server_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use validator::Validate;

/// # Edit Server
///
/// Edit a server by its id.
#[openapi(tag = "Server Information")]
#[patch("/<target>", data = "<data>")]
pub async fn edit(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    data: Json<v0::DataEditServer>,
) -> Result<Json<v0::Server>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let mut server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    let permissions = calculate_server_permissions(&mut query).await;

    // Check permissions
    if data.name.is_none()
        && data.description.is_none()
        && data.icon.is_none()
        && data.banner.is_none()
        && data.system_messages.is_none()
        && data.categories.is_none()
        // && data.nsfw.is_none()
        && data.flags.is_none()
        && data.analytics.is_none()
        && data.discoverable.is_none()
        && data.remove.is_empty()
    {
        return Ok(Json(server.into()));
    } else if data.name.is_some()
        || data.description.is_some()
        || data.icon.is_some()
        || data.banner.is_some()
        || data.system_messages.is_some()
        || data.analytics.is_some()
        || !data.remove.is_empty()
    {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::ManageServer)?;
    }

    // Check we are privileged if changing sensitive fields
    if (data.flags.is_some() /*|| data.nsfw.is_some()*/ || data.discoverable.is_some())
        && !user.privileged
    {
        return Err(create_error!(NotPrivileged));
    }

    // Changing categories requires manage channel
    if data.categories.is_some() {
        permissions.throw_if_lacking_channel_permission(ChannelPermission::ManageChannel)?;
    }

    let v0::DataEditServer {
        name,
        description,
        icon,
        banner,
        categories,
        system_messages,
        flags,
        // nsfw,
        discoverable,
        analytics,
        remove,
    } = data;

    let mut partial = PartialServer {
        name,
        description,
        categories: categories.map(|v| v.into_iter().map(Into::into).collect()),
        system_messages: system_messages.map(Into::into),
        flags,
        // nsfw,
        discoverable,
        analytics,
        ..Default::default()
    };

    // 1. Remove fields from object
    if remove.contains(&v0::FieldsServer::Banner) {
        if let Some(banner) = &server.banner {
            db.mark_attachment_as_deleted(&banner.id).await?;
        }
    }

    if remove.contains(&v0::FieldsServer::Icon) {
        if let Some(icon) = &server.icon {
            db.mark_attachment_as_deleted(&icon.id).await?;
        }
    }

    // 2. Validate changes
    if let Some(system_messages) = &partial.system_messages {
        for id in system_messages.clone().into_channel_ids() {
            if !server.channels.contains(&id) {
                return Err(create_error!(NotFound));
            }
        }
    }

    if let Some(categories) = &mut partial.categories {
        let mut channel_ids = HashSet::new();
        for category in categories {
            for channel in &category.channels {
                if channel_ids.contains(channel) {
                    return Err(create_error!(InvalidOperation));
                }

                channel_ids.insert(channel.to_string());
            }

            category
                .channels
                .retain(|item| server.channels.contains(item));
        }
    }

    // 3. Apply new icon
    if let Some(icon) = icon {
        partial.icon = Some(File::use_server_icon(db, &icon, &server.id, &user.id).await?);
        server.icon = partial.icon.clone();
    }

    // 4. Apply new banner
    if let Some(banner) = banner {
        partial.banner = Some(File::use_server_banner(db, &banner, &server.id, &user.id).await?);
        server.banner = partial.banner.clone();
    }

    server
        .update(db, partial, remove.into_iter().map(Into::into).collect())
        .await?;

    Ok(Json(server.into()))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    #[rocket::async_test]
    async fn test_edit_empty_request() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let returned_server: v0::Server = response.into_json().await.expect("`Server`");
        assert_eq!(returned_server.id, server.id);
    }

    #[rocket::async_test]
    async fn test_edit_with_name() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let new_name = "Updated Server Name";
        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(format!(r#"{{"name": "{}"}}"#, new_name))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let returned_server: v0::Server = response.into_json().await.expect("`Server`");
        assert_eq!(returned_server.name, new_name);
    }

    #[rocket::async_test]
    async fn test_edit_with_description() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let new_description = "This is an updated description";
        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(format!(r#"{{"description": "{}"}}"#, new_description))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let returned_server: v0::Server = response.into_json().await.expect("`Server`");
        assert_eq!(
            returned_server.description,
            Some(new_description.to_string())
        );
    }

    #[rocket::async_test]
    async fn test_edit_with_icon() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let icon_id = "test_icon_id";
        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(format!(r#"{{"icon": "{}"}}"#, icon_id))
            .dispatch()
            .await;

        assert!(response.status() == Status::Ok || response.status() == Status::NotFound);
    }

    #[rocket::async_test]
    async fn test_edit_with_banner() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let banner_id = "test_banner_id";
        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(format!(r#"{{"banner": "{}"}}"#, banner_id))
            .dispatch()
            .await;

        assert!(response.status() == Status::Ok || response.status() == Status::NotFound);
    }

    #[rocket::async_test]
    async fn test_edit_with_system_messages() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;

        let channel_id = &channels[0].id();
        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(format!(
                r#"{{"system_messages": {{"user_joined": "{}"}}}}"#,
                channel_id
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_edit_with_categories() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;

        let channel_id = &channels[0].id();
        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(format!(
                r#"{{"categories": [{{"id": "cat1", "title": "Category 1", "channels": ["{}"]}}]}}"#,
                channel_id
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_edit_with_flags_privileged() {
        let harness = TestHarness::new().await;
        let (_, session, mut user) = harness.new_user().await;

        user.privileged = true;
        user.update(
            &harness.db,
            revolt_database::PartialUser {
                privileged: Some(true),
                ..Default::default()
            },
            vec![],
        )
        .await
        .unwrap();

        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"flags": 1}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_edit_with_analytics() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"analytics": true}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_edit_with_discoverable_privileged() {
        let harness = TestHarness::new().await;
        let (_, session, mut user) = harness.new_user().await;

        user.privileged = true;
        user.update(
            &harness.db,
            revolt_database::PartialUser {
                privileged: Some(true),
                ..Default::default()
            },
            vec![],
        )
        .await
        .unwrap();

        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"discoverable": true}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_edit_with_remove_icon() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"remove": ["Icon"]}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_edit_flags_without_privilege() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"flags": 1}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn test_edit_discoverable_without_privilege() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"discoverable": true}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn test_edit_system_messages_invalid_channel() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"system_messages": {"user_joined": "invalid_channel_id"}}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn test_edit_categories_duplicate_channel() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, channels) = harness.new_server(&user).await;

        let channel_id = &channels[0].id();
        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(format!(
                r#"{{"categories": [
                    {{"id": "cat1", "title": "Category 1", "channels": ["{}"]}},
                    {{"id": "cat2", "title": "Category 2", "channels": ["{}"]}}
                ]}}"#,
                channel_id, channel_id
            ))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    async fn test_remove_existing_banner() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"remove": ["Banner"]}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_edit_without_privilege_non_sensitive() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"name": "New Name Without Privilege"}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn test_edit_flags_and_discoverable_without_privilege() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;
        let (server, _) = harness.new_server(&user).await;

        let response = harness
            .client
            .patch(format!("/servers/{}", server.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(rocket::http::ContentType::JSON)
            .body(r#"{"flags": 1, "discoverable": true}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }
}
