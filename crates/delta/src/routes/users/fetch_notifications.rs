use revolt_database::{Database, NotificationCenter, User, InboxOptions};
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
) -> Result<Json<Vec<v0::NotificationCenter>>> {
    let items = NotificationCenter::fetch_inbox(
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

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{Channel, Member, Server, User};
    use revolt_models::v0::{self, DataCreateGroup};
    use rocket::http::{ContentType, Header, Status};
    use serde_json::json;
    use ulid::Ulid;

    // NOTE: adjust to wherever this route is mounted in delta
    const ROUTE: &str = "/users/notifications";

    /// A logged-in test user: session token + user
    struct Actor {
        token: String,
        user: User,
    }

    async fn actor(harness: &TestHarness) -> Actor {
        let (_, session, user) = harness.new_user().await;
        Actor {
            token: session.token.to_string(),
            user,
        }
    }

    /// Owner + member in a group DM. Enough for direct-mention tests,
    /// since those are read straight from the notifications collection.
    async fn group_setup(harness: &TestHarness) -> (Actor, Actor, Channel) {
        let owner = actor(harness).await;
        let member = actor(harness).await;

        let group = Channel::create_group(
            &harness.db,
            DataCreateGroup {
                users: [member.user.id.clone()].into_iter().collect(),
                ..Default::default()
            },
            owner.user.id.clone(),
        )
        .await
        .expect("`Channel`");

        (owner, member, group)
    }

    /// Owner + member in a server. Needed for @everyone / role tests,
    /// which resolve channels through server membership.
    async fn server_setup(harness: &TestHarness) -> (Actor, Actor, Channel) {
        let owner = actor(harness).await;
        let member = actor(harness).await;

        let (server, channels) = Server::create(
            &harness.db,
            v0::DataCreateServer {
                name: "Inbox Test".to_string(),
                description: None,
                nsfw: None,
            },
            &owner.user,
            true,
        )
        .await
        .expect("`Server`");

        Member::create(&harness.db, &server, &member.user, Some(channels.clone()))
            .await
            .expect("`Member`");

        let channel = channels.into_iter().next().expect("text channel");
        (owner, member, channel)
    }

    async fn send(
        harness: &TestHarness,
        from: &Actor,
        channel: &Channel,
        content: String,
    ) -> v0::Message {
        let response = harness
            .client
            .post(format!("/channels/{}/messages", channel.id()))
            .header(Header::new("x-session-token", from.token.clone()))
            .header(Header::new("Idempotency-Key", Ulid::new().to_string()))
            .header(ContentType::JSON)
            .body(json!({ "content": content }).to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        response.into_json().await.expect("`Message`")
    }

    async fn fetch(
        harness: &TestHarness,
        as_user: &Actor,
        query: &str,
    ) -> (Status, Vec<v0::NotificationCenter>) {
        let response = harness
            .client
            .get(format!("{ROUTE}{query}"))
            .header(Header::new("x-session-token", as_user.token.clone()))
            .dispatch()
            .await;

        let status = response.status();
        let body = if status == Status::Ok {
            response.into_json().await.expect("`Vec<NotificationCenter>`")
        } else {
            vec![]
        };
        (status, body)
    }

    #[rocket::async_test]
    async fn requires_authentication() {
        let harness = TestHarness::new().await;

        let response = harness.client.get(ROUTE).dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    async fn empty_inbox_returns_empty_list() {
        let harness = TestHarness::new().await;
        let user = actor(&harness).await;

        let (status, items) = fetch(&harness, &user, "").await;
        assert_eq!(status, Status::Ok);
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn direct_mention_appears_in_inbox() {
        let harness = TestHarness::new().await;
        let (owner, member, group) = group_setup(&harness).await;

        let message = send(&harness, &owner, &group, format!("hey <@{}>", member.user.id)).await;

        let (status, items) = fetch(&harness, &member, "").await;
        assert_eq!(status, Status::Ok);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, v0::NotificationKind::Mention);
        assert_eq!(items[0].message_id, message.id);
        assert_eq!(items[0].channel_id, group.id());
        assert_eq!(items[0].message.id, message.id);
    }

    #[rocket::async_test]
    async fn author_does_not_notify_themselves() {
        let harness = TestHarness::new().await;
        let (owner, _member, group) = group_setup(&harness).await;

        send(&harness, &owner, &group, format!("note to self <@{}>", owner.user.id)).await;

        let (status, items) = fetch(&harness, &owner, "").await;
        assert_eq!(status, Status::Ok);
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn mentions_filter_hides_direct_mentions() {
        let harness = TestHarness::new().await;
        let (owner, member, group) = group_setup(&harness).await;

        send(&harness, &owner, &group, format!("hi <@{}>", member.user.id)).await;

        let (_, shown) = fetch(&harness, &member, "?mentions=true&roles=false&everyone=false").await;
        assert_eq!(shown.len(), 1);

        let (status, hidden) = fetch(&harness, &member, "?mentions=false&roles=false&everyone=false").await;
        assert_eq!(status, Status::Ok);
        assert!(hidden.is_empty());
    }

    #[rocket::async_test]
    async fn results_are_newest_first_and_limited() {
        let harness = TestHarness::new().await;
        let (owner, member, group) = group_setup(&harness).await;

        let mut ids = vec![];
        for i in 0..3 {
            let m = send(&harness, &owner, &group, format!("#{i} <@{}>", member.user.id)).await;
            ids.push(m.id);
        }
        ids.reverse(); // newest first

        let (_, all) = fetch(&harness, &member, "").await;
        let got: Vec<_> = all.iter().map(|i| i.message_id.clone()).collect();
        assert_eq!(got, ids);

        let (_, limited) = fetch(&harness, &member, "?limit=2").await;
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].message_id, ids[0]);
        assert_eq!(limited[1].message_id, ids[1]);
    }

    #[rocket::async_test]
    async fn before_paginates_to_older_items() {
        let harness = TestHarness::new().await;
        let (owner, member, group) = group_setup(&harness).await;

        let mut ids = vec![];
        for i in 0..3 {
            let m = send(&harness, &owner, &group, format!("#{i} <@{}>", member.user.id)).await;
            ids.push(m.id);
        }

        let (status, page) = fetch(
            &harness,
            &member,
            &format!("?mentions=true&roles=false&everyone=false&before={}", ids[2]),
        )
        .await;

        assert_eq!(status, Status::Ok);
        let got: Vec<_> = page.iter().map(|i| i.message_id.clone()).collect();
        assert_eq!(got, vec![ids[1].clone(), ids[0].clone()]);
    }

    #[rocket::async_test]
    async fn deleted_message_removes_notification() {
        let harness = TestHarness::new().await;
        let (owner, member, group) = group_setup(&harness).await;

        let message = send(&harness, &owner, &group, format!("oops <@{}>", member.user.id)).await;

        let response = harness
            .client
            .delete(format!("/channels/{}/messages/{}", group.id(), message.id))
            .header(Header::new("x-session-token", owner.token.clone()))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NoContent);

        let (status, items) = fetch(&harness, &member, "").await;
        assert_eq!(status, Status::Ok);
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn everyone_mention_respects_filter() {
        let harness = TestHarness::new().await;
        let (owner, member, channel) = server_setup(&harness).await;

        let message = send(&harness, &owner, &channel, "@everyone standup".to_string()).await;

        let (_, shown) = fetch(&harness, &member, "?mentions=false&roles=false&everyone=true").await;
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].kind, v0::NotificationKind::Everyone);
        assert_eq!(shown[0].message_id, message.id);

        let (_, hidden) = fetch(&harness, &member, "?mentions=true&roles=true&everyone=false").await;
        assert!(hidden.is_empty());
    }

    #[rocket::async_test]
    async fn mention_and_everyone_on_same_message_is_deduplicated() {
        let harness = TestHarness::new().await;
        let (owner, member, channel) = server_setup(&harness).await;

        send(&harness, &owner, &channel, format!("@everyone and <@{}>", member.user.id)).await;

        let (_, items) = fetch(&harness, &member, "").await;
        assert_eq!(items.len(), 1);
        // InboxKind orders Mention first, so it wins the dedup
        assert_eq!(items[0].kind, v0::NotificationKind::Mention);
    }
}
