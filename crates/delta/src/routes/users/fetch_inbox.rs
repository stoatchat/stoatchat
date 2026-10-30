use revolt_database::{Database, InboxEntry, User, InboxOptions};
use revolt_models::v0;
use revolt_result::{Result, create_error};
use rocket::{serde::json::Json, State};

/// # Fetch User Inbox
///
/// Retrieve a user's inbox items
#[openapi(tag = "User Information")]
#[get("/inbox?<options..>")]
pub async fn fetch_inbox(
    db: &State<Database>,
    user: User,
    options: v0::OptionsFetchInbox,
) -> Result<Json<v0::InboxResponse>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let page = InboxEntry::fetch_inbox(
        db,
        &user,
        InboxOptions {
            limit: options.limit.unwrap_or(50).clamp(1, 100),
            before: options.before,
            mentions: options.mentions.unwrap_or(true),
            roles: options.roles.unwrap_or(true),
            everyone: options.everyone.unwrap_or(true),
        },
    )
    .await?;

    Ok(Json(v0::InboxResponse {
            items: page.items.into_iter().map(Into::into).collect(),
            pagination: page.pagination,
        }))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{
        util::reference::Reference, Channel, Member, PartialMember, Role, Server, User,
    };
    use revolt_models::v0::{self, DataCreateGroup};
    use rocket::http::{ContentType, Header, Status};
    use serde_json::json;
    use std::collections::HashMap;
    use ulid::Ulid;

    const ROUTE: &str = "/users/inbox";

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

    /// Server with a role, both members joined, and the role assigned to the
    /// non-owner member so role mentions address them.
    async fn server_with_role_setup(harness: &TestHarness) -> (Actor, Actor, Channel, Role) {
        let owner = actor(harness).await;
        let member = actor(harness).await;

        let (server, channels) = Server::create(
            &harness.db,
            v0::DataCreateServer {
                name: "Inbox Role Test".to_string(),
                description: None,
                nsfw: None,
            },
            &owner.user,
            true,
        )
        .await
        .expect("`Server`");

        let role = Role::create(&harness.db, &server, "Mentionable".to_string())
            .await
            .expect("`Role`");

        Member::create(&harness.db, &server, &member.user, Some(channels.clone()))
            .await
            .expect("`Member`");

        let mut member_ref = Reference::from_unchecked(&member.user.id)
            .as_member(&harness.db, &server.id)
            .await
            .expect("`Member`");
        member_ref
            .update(
                &harness.db,
                PartialMember {
                    roles: Some(vec![role.id.clone()]),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("`Member`");

        let channel = channels.into_iter().next().expect("text channel");
        (owner, member, channel, role)
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

    async fn send_reply(
        harness: &TestHarness,
        from: &Actor,
        channel: &Channel,
        content: String,
        reply_to: &str,
        mention: bool,
    ) -> v0::Message {
        let response = harness
            .client
            .post(format!("/channels/{}/messages", channel.id()))
            .header(Header::new("x-session-token", from.token.clone()))
            .header(Header::new("Idempotency-Key", Ulid::new().to_string()))
            .header(ContentType::JSON)
            .body(
                json!({
                    "content": content,
                    "replies": [{ "id": reply_to, "mention": mention }]
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        response.into_json().await.expect("`Message`")
    }

    async fn edit(
        harness: &TestHarness,
        from: &Actor,
        channel: &Channel,
        message: &v0::Message,
        content: String,
    ) -> v0::Message {
        let response = harness
            .client
            .patch(format!("/channels/{}/messages/{}", channel.id(), message.id))
            .header(Header::new("x-session-token", from.token.clone()))
            .header(ContentType::JSON)
            .body(json!({ "content": content }).to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        response.into_json().await.expect("`Message`")
    }

    async fn fetch_page(
        harness: &TestHarness,
        as_user: &Actor,
        query: &str,
    ) -> (Status, Option<v0::InboxResponse>) {
        let response = harness
            .client
            .get(format!("{ROUTE}{query}"))
            .header(Header::new("x-session-token", as_user.token.clone()))
            .dispatch()
            .await;

        let status = response.status();
        let body = if status == Status::Ok {
            Some(response.into_json().await.expect("`InboxResponse`"))
        } else {
            None
        };
        (status, body)
    }

    async fn fetch(harness: &TestHarness, as_user: &Actor, query: &str) -> (Status, Vec<v0::InboxEntry>) {
        let (status, page) = fetch_page(harness, as_user, query).await;
        (status, page.map(|p| p.items).unwrap_or_default())
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
        assert_eq!(items[0].entry_kind, v0::InboxKind::Mention);
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
        assert_eq!(shown[0].entry_kind, v0::InboxKind::Everyone);
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
        assert_eq!(items[0].entry_kind, v0::InboxKind::Mention);
    }

    #[rocket::async_test]
    async fn pagination_is_set_until_exhausted() {
        let harness = TestHarness::new().await;
        let (owner, member, group) = group_setup(&harness).await;

        for i in 0..3 {
            send(&harness, &owner, &group, format!("#{i} <@{}>", member.user.id)).await;
        }

        let (_, page) = fetch_page(&harness, &member, "?limit=2").await;
        let page = page.expect("page");
        assert_eq!(page.items.len(), 2);
        let cursor = page.pagination.expect("more pages");

        let (_, page) = fetch_page(&harness, &member, &format!("?limit=2&before={cursor}")).await;
        let page = page.expect("page");
        assert_eq!(page.items.len(), 1);
        assert!(page.pagination.is_none());
    }

    #[rocket::async_test]
    async fn role_mention_appears_and_edit_removes_it() {
        let harness = TestHarness::new().await;
        let (owner, member, channel, role) = server_with_role_setup(&harness).await;

        let message = send(&harness, &owner, &channel, format!("<%{}>", role.id)).await;

        let (_, items) = fetch(&harness, &member, "?mentions=false&roles=true&everyone=false").await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].entry_kind, v0::InboxKind::Role);
        assert_eq!(items[0].message_id, message.id);

        // Editing the role mention out should drop it from the inbox
        edit(&harness, &owner, &channel, &message, "nothing here".to_string()).await;

        let (_, items) = fetch(&harness, &member, "?mentions=false&roles=true&everyone=false").await;
        assert!(
            items.is_empty(),
            "role mention still returned after being edited out"
        );
    }

    #[rocket::async_test]
    async fn everyone_mention_edit_removes_it() {
        let harness = TestHarness::new().await;
        let (owner, member, channel) = server_setup(&harness).await;

        let message = send(&harness, &owner, &channel, "@everyone standup".to_string()).await;

        let (_, items) = fetch(&harness, &member, "?mentions=false&roles=false&everyone=true").await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].entry_kind, v0::InboxKind::Everyone);

        // Editing @everyone out should drop it from the inbox
        edit(&harness, &owner, &channel, &message, "standup".to_string()).await;

        let (_, items) = fetch(&harness, &member, "?mentions=false&roles=false&everyone=true").await;
        assert!(
            items.is_empty(),
            "@everyone still returned after being edited out"
        );
    }

    #[rocket::async_test]
    async fn adding_role_mention_via_edit_appears_in_inbox() {
        let harness = TestHarness::new().await;
        let (owner, member, channel, _role) = server_with_role_setup(&harness).await;

        let message = send(&harness, &owner, &channel, "plain text".to_string()).await;

        let (_, items) = fetch(&harness, &member, "?mentions=false&roles=true&everyone=false").await;
        assert!(items.is_empty());

        // Adding a role mention via edit should surface it in the inbox
        edit(&harness, &owner, &channel, &message, format!("<@{}>", member.user.id)).await;
        let (_, items) = fetch(&harness, &member, "?mentions=true&roles=false&everyone=false").await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].entry_kind, v0::InboxKind::Mention);
    }

    #[rocket::async_test]
    async fn message_edit_can_add_everyone_mention() {
        let harness = TestHarness::new().await;
        let (owner, member, channel) = server_setup(&harness).await;

        let message = send(&harness, &owner, &channel, "plain text".to_string()).await;

        let (_, items) = fetch(&harness, &member, "?mentions=false&roles=false&everyone=true").await;
        assert!(items.is_empty());

        // Owner has MentionEveryone, so the added mention should go through
        edit(&harness, &owner, &channel, &message, "@everyone ping".to_string()).await;

        let (_, items) = fetch(&harness, &member, "?mentions=false&roles=false&everyone=true").await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].entry_kind, v0::InboxKind::Everyone);
    }

    #[rocket::async_test]
    async fn non_pinging_reply_does_not_keep_edited_out_mention() {
        let harness = TestHarness::new().await;
        let (owner, member, group) = group_setup(&harness).await;

        let target = send(&harness, &member, &group, "original".to_string()).await;

        // Reply without pinging, but also explicitly mention the member
        let message = send_reply(
            &harness,
            &owner,
            &group,
            format!("<@{}>", member.user.id),
            &target.id,
            false,
        )
        .await;

        let (_, items) = fetch(&harness, &member, "").await;
        assert_eq!(items.len(), 1);

        // Editing the explicit mention out must drop the notification: the
        // non-pinging reply cannot keep it alive.
        edit(&harness, &owner, &group, &message, "just replying".to_string()).await;

        let (_, items) = fetch(&harness, &member, "").await;
        assert!(
            items.is_empty(),
            "non-pinging reply kept an edited-out mention in the inbox"
        );
    }

    #[rocket::async_test]
    async fn pinging_reply_keeps_mention_through_edit() {
        let harness = TestHarness::new().await;
        let (owner, member, group) = group_setup(&harness).await;

        let target = send(&harness, &member, &group, "original".to_string()).await;

        // Reply with a ping and no explicit mention in the content
        let message =
            send_reply(&harness, &owner, &group, "hello".to_string(), &target.id, true).await;

        let (_, items) = fetch(&harness, &member, "").await;
        assert_eq!(items.len(), 1);

        // The reply still mentions the member, so editing the body keeps it
        edit(&harness, &owner, &group, &message, "hello there".to_string()).await;

        let (_, items) = fetch(&harness, &member, "").await;
        assert_eq!(
            items.len(),
            1,
            "pinging reply lost its mention after an unrelated edit"
        );
    }

    #[rocket::async_test]
    async fn inbox_hidden_without_read_message_history() {
        use revolt_database::PartialChannel;
        use revolt_permissions::{ChannelPermission, OverrideField};

        let harness = TestHarness::new().await;
        let owner = actor(&harness).await;
        let member = actor(&harness).await;

        let (server, channels) = Server::create(
            &harness.db,
            v0::DataCreateServer {
                name: "Inbox History Test".to_string(),
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

        let mut channel = channels.into_iter().next().expect("text channel");

        // Member can still view the channel, but history is denied for them
        let member_role = Role::create(&harness.db, &server, "No History".to_string())
            .await
            .expect("`Role`");
        let mut overrides = HashMap::new();
        overrides.insert(
            member_role.id.clone(),
            OverrideField {
                a: 0,
                d: (ChannelPermission::ReadMessageHistory) as i64,
            },
        );
        channel
            .update(
                &harness.db,
                PartialChannel {
                    role_permissions: Some(overrides),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("`Channel`");

        let mut member_ref = Reference::from_unchecked(&member.user.id)
            .as_member(&harness.db, &server.id)
            .await
            .expect("`Member`");
        member_ref
            .update(
                &harness.db,
                PartialMember {
                    roles: Some(vec![member_role.id.clone()]),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("`Member`");

        send(&harness, &owner, &channel, "@everyone secret".to_string()).await;

        // The inbox must not leak messages the user cannot read the history of
        let (_, items) = fetch(&harness, &member, "?mentions=false&roles=false&everyone=true").await;
        assert!(
            items.is_empty(),
            "inbox leaked a message without ReadMessageHistory"
        );
    }
}
