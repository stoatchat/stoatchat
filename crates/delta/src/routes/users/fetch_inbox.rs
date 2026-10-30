use std::collections::{HashMap, HashSet};

use revolt_database::{
    util::permissions::DatabasePermissionQuery, Channel, Database, InboxEntry, InboxOptions,
    Member, Message, Server, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use ulid::Ulid;

/// Max DB pages scanned per request when permission/deletion filtering
/// leaves a page short.
const MAX_SCAN_ROUNDS: usize = 5;

/// Loads channels/servers on demand and caches view-permission results.
struct Visibility<'a> {
    db: &'a Database,
    user: &'a User,
    members: HashMap<String, Member>,
    servers: HashMap<String, Server>,
    channels: HashMap<String, Channel>,
    allowed: HashMap<String, bool>,
}

impl<'a> Visibility<'a> {
    /// Fetch every channel/server the given entries need that we don't have yet.
    async fn prepare(&mut self, entries: &[InboxEntry]) -> Result<()> {
        let missing: Vec<String> = entries
            .iter()
            .map(|e| e.channel_id.clone())
            .filter(|id| !self.channels.contains_key(id))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if !missing.is_empty() {
            for channel in self.db.fetch_channels(&missing).await? {
                self.channels.insert(channel.id().to_string(), channel);
            }
        }

        let missing_servers: Vec<String> = entries
            .iter()
            .filter_map(|e| self.channels.get(&e.channel_id)?.server().map(str::to_string))
            .filter(|id| !self.servers.contains_key(id))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if !missing_servers.is_empty() {
            for server in self.db.fetch_servers(&missing_servers).await? {
                self.servers.insert(server.id.clone(), server);
            }
        }

        Ok(())
    }

    async fn can_view(&mut self, channel_id: &str) -> bool {
        if let Some(allowed) = self.allowed.get(channel_id) {
            return *allowed;
        }
        let allowed = self.check(channel_id).await;
        self.allowed.insert(channel_id.to_string(), allowed);
        allowed
    }

    async fn check(&self, channel_id: &str) -> bool {
        let Some(channel) = self.channels.get(channel_id) else {
            return false;
        };

        let base = DatabasePermissionQuery::new(self.db, self.user).channel(channel);
        let mut query = match channel.server() {
            Some(server_id) => match (self.servers.get(server_id), self.members.get(server_id)) {
                (Some(server), Some(member)) => base.server(server).member(member),
                _ => return false,
            },
            None => base,
        };

        let permissions = calculate_channel_permissions(&mut query).await;
        permissions.has_channel_permission(ChannelPermission::ViewChannel)
            && permissions.has_channel_permission(ChannelPermission::ReadMessageHistory)
    }
}

/// # Fetch User Inbox
///
/// Retrieve a user's inbox items, newest first
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
    let db: &Database = db.inner();

    let limit = options.limit.unwrap_or(50).clamp(1, 100);

    // The cursor is just a message id.
    if let Some(before) = &options.before {
        Ulid::from_string(before).map_err(|_| create_error!(InvalidOperation))?;
    }

    let members: HashMap<String, Member> = db
        .fetch_all_memberships(&user.id)
        .await?
        .into_iter()
        .map(|member| (member.id.server.clone(), member))
        .collect();

    let empty = || Ok(Json(v0::InboxResponse { items: vec![], pagination: None }));

    if options.server_id.as_ref().is_some_and(|id| !members.contains_key(id)) {
        return empty();
    }

    let mut opts = InboxOptions {
        limit,
        before: options.before,
        mentions: options.mentions.unwrap_or(true),
        roles: options.roles.unwrap_or(true),
        everyone: options.everyone.unwrap_or(true),
        server_id: options.server_id,
    };
    if !(opts.mentions || opts.roles || opts.everyone) {
        return empty();
    }

    let mut vis = Visibility {
        db,
        user: &user,
        members,
        servers: HashMap::new(),
        channels: HashMap::new(),
        allowed: HashMap::new(),
    };

    let mut page: Vec<(InboxEntry, Message)> = vec![];
    let mut exhausted = false;

    for _ in 0..MAX_SCAN_ROUNDS {
        let entries = db.fetch_inbox_entries(&user.id, &opts).await?;
        exhausted = (entries.len() as i64) < limit;

        let Some(last) = entries.last() else {
            break;
        };
        opts.before = Some(last.message_id.clone());

        vis.prepare(&entries).await?;

        let ids: Vec<String> = entries.iter().map(|e| e.message_id.clone()).collect();
        let mut messages: HashMap<String, Message> = db
            .fetch_messages_by_id(&ids)
            .await?
            .into_iter()
            .map(|message| (message.id.clone(), message))
            .collect();

        for entry in entries {
            if !vis.can_view(&entry.channel_id).await {
                continue;
            }
            // Deleted messages leave orphan entries until TTL; skip them.
            if let Some(message) = messages.remove(&entry.message_id) {
                page.push((entry, message));
            }
        }

        if exhausted || page.len() >= limit as usize {
            break;
        }
    }

    let more = !exhausted || page.len() > limit as usize;
    page.truncate(limit as usize);

    let pagination = if !more {
        None
    } else if page.len() == limit as usize {
        page.last().map(|(entry, _)| entry.message_id.clone())
    } else {
        opts.before
    };

    let items = page
        .into_iter()
        .map(|(entry, message)| v0::InboxEntry {
            entry_kind: entry.kind.into(),
            message_id: entry.message_id,
            channel_id: entry.channel_id,
            server_id: entry.server_id,
            message: message.into_model(None, None),
        })
        .collect();

    Ok(Json(v0::InboxResponse { items, pagination }))
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{util::idempotency::IdempotencyKey, Channel, InboxEntry, InboxKind, InboxOptions, Member, Message, PartialMember, RemovalIntention, Server, User, MessageFlagsValue};
    use revolt_models::v0::{self, MessageFlags};
    use rocket::{
        http::{Header, Status},
        tokio::time::sleep,
    };


    async fn fetch(harness: &TestHarness, token: &str, query: &str) -> v0::InboxResponse {
        let response = harness
            .client
            .get(format!("/users/inbox{query}"))
            .header(Header::new("x-session-token", token.to_string()))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        response.into_json().await.expect("inbox response")
    }

    async fn fetch_until(harness: &TestHarness, token: &str, query: &str, expected: usize) -> v0::InboxResponse {
        for _ in 0..40 {
            let page = fetch(harness, token, query).await;
            if page.items.len() >= expected {
                return page;
            }
            sleep(Duration::from_millis(50)).await;
        }
        fetch(harness, token, query).await
    }

    fn db_opts() -> InboxOptions {
        InboxOptions { limit: 50, before: None, mentions: true, roles: true, everyone: true, server_id: None }
    }

    /// The server owner sends (owners can mention everyone/roles without extra setup).
    async fn send(harness: &TestHarness, channel: &Channel, author: &User, content: &str, flags: Option<u32>) -> Message {
        Message::create_from_api(
            &harness.db,
            Some(&harness.amqp),
            channel.clone(),
            v0::DataMessageSend {
                content: Some(content.to_string()),
                nonce: None,
                attachments: None,
                replies: None,
                embeds: None,
                masquerade: None,
                interactions: None,
                flags,
            },
            v0::MessageAuthor::User(&author.clone().into(&harness.db, Some(author)).await),
            Some(author.clone().into(&harness.db, Some(author)).await),
            None,
            author.limits().await,
            IdempotencyKey::unchecked_from_string(ulid::Ulid::new().to_string()),
            false,
            true,
        )
            .await
            .expect("Failed to send message")
    }

    /// Create a user and join them to the server, optionally with a role.
    async fn join(harness: &TestHarness, server: &Server, role: Option<&str>) -> (User, String, Member) {
        let (_, session, user) = harness.new_user().await;
        let (mut member, _) = Member::create(&harness.db, server, &user, None)
            .await
            .expect("Failed to join server");

        if let Some(role) = role {
            member
                .update(
                    &harness.db,
                    PartialMember { roles: Some(vec![role.to_string()]), ..Default::default() },
                    vec![],
                )
                .await
                .expect("Failed to grant role");
        }

        (user, session.token, member)
    }

    #[rocket::async_test]
    async fn inbox_kinds_and_priority() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (server, _) = harness.new_server(&owner).await;
        let channel = harness.new_channel(&server).await;
        let role = harness.new_role(&server, 1, None).await;

        let (alice, alice_token, _) = join(&harness, &server, Some(&role.id)).await; // mentioned + role
        let (_, carol_token, _) = join(&harness, &server, Some(&role.id)).await;     // role only
        let (_, bob_token, _) = join(&harness, &server, None).await;                 // everyone only

        send(&harness, &channel, &owner, &format!("<@{}> <%{}> @everyone", alice.id, role.id), None).await;

        let alice_inbox = fetch_until(&harness, &alice_token, "", 1).await;
        assert_eq!(alice_inbox.items.len(), 1);
        assert_eq!(alice_inbox.items[0].entry_kind, v0::InboxKind::Mention);

        let carol_inbox = fetch_until(&harness, &carol_token, "", 1).await;
        assert_eq!(carol_inbox.items.len(), 1);
        assert_eq!(carol_inbox.items[0].entry_kind, v0::InboxKind::Role);

        let bob_inbox = fetch_until(&harness, &bob_token, "", 1).await;
        assert_eq!(bob_inbox.items.len(), 1);
        assert_eq!(bob_inbox.items[0].entry_kind, v0::InboxKind::Everyone);

        // Kind filters apply to the stored kind: Bob's entry is Everyone only.
        assert!(fetch(&harness, &bob_token, "?everyone=false").await.items.is_empty());
        assert!(fetch(&harness, &carol_token, "?roles=false").await.items.is_empty());

        assert!(harness
            .db
            .fetch_inbox_entries(&owner.id, &db_opts())
            .await
            .unwrap()
            .is_empty());
    }

    #[rocket::async_test]
    async fn inbox_pagination_returns_everything_once() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (server, _) = harness.new_server(&owner).await;
        let channel = harness.new_channel(&server).await;
        let (alice, token, _) = join(&harness, &server, None).await;

        let mut sent = vec![];
        for i in 0..5 {
            sent.push(send(&harness, &channel, &owner, &format!("<@{}> {i}", alice.id), None).await.id);
        }
        // ULIDs created in the same millisecond aren't ordered by send order.
        sent.sort_by(|a, b| b.cmp(a));

        fetch_until(&harness, &token, "?limit=100", 5).await;

        let mut seen = vec![];
        let mut query = "?limit=2".to_string();
        for _ in 0..10 {
            let page = fetch(&harness, &token, &query).await;
            assert!(page.items.len() <= 2);
            seen.extend(page.items.iter().map(|i| i.message_id.clone()));

            match page.pagination {
                Some(cursor) => query = format!("?limit=2&before={cursor}"),
                None => break,
            }
        }

        assert_eq!(seen, sent);
    }

    #[rocket::async_test]
    async fn inbox_rejects_invalid_cursor() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let response = harness
            .client
            .get("/users/inbox?before=not-a-ulid")
            .header(Header::new("x-session-token", session.token))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    async fn inbox_silent_messages_create_no_entries() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (server, _) = harness.new_server(&owner).await;
        let channel = harness.new_channel(&server).await;
        let (alice, token, _) = join(&harness, &server, None).await;

        let mut flags = MessageFlagsValue(0);
        flags.set(MessageFlags::SuppressNotifications, true);
        let silent = send(
            &harness,
            &channel,
            &owner,
            &format!("<@{}> shh", alice.id),
            Some(flags.0),
        ).await;

        assert!(MessageFlagsValue(silent.flags.unwrap_or(0)).has(MessageFlags::SuppressNotifications));

        sleep(Duration::from_millis(500)).await;

        let page = fetch(&harness, &token, "").await;
        assert!(page.items.iter().all(|i| i.message_id != silent.id));
    }

    #[rocket::async_test]
    async fn inbox_entries_removed_with_message_and_membership() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (server, _) = harness.new_server(&owner).await;
        let channel = harness.new_channel(&server).await;
        let (alice, token, alice_member) = join(&harness, &server, None).await;

        // Deleting the message removes its entry.
        let message = send(&harness, &channel, &owner, &format!("<@{}> a", alice.id), None).await;
        fetch_until(&harness, &token, "", 1).await;
        message.clone().delete(&harness.db).await.expect("Failed to delete message");
        assert!(harness.db.fetch_inbox_entries(&alice.id, &db_opts()).await.unwrap().is_empty());

        // Leaving the server removes that server's entries.
        send(&harness, &channel, &owner, &format!("<@{}> b", alice.id), None).await;
        fetch_until(&harness, &token, "", 1).await;
        alice_member
            .remove(&harness.db, &server, RemovalIntention::Leave, true)
            .await
            .expect("Failed to remove member");
        assert!(harness.db.fetch_inbox_entries(&alice.id, &db_opts()).await.unwrap().is_empty());
        assert!(fetch(&harness, &token, "").await.items.is_empty());
    }

    #[rocket::async_test]
    async fn inbox_hides_entries_for_servers_the_user_is_not_in() {
        let harness = TestHarness::new().await;
        let (_, _, other_owner) = harness.new_user().await;
        let (other_server, other_channels) = harness.new_server(&other_owner).await;
        let (_, _, message) = harness.new_message(&other_owner, &other_server, other_channels).await;

        // A stray entry (e.g. left over after losing access) must not leak.
        let (_, session, alice) = harness.new_user().await;
        harness
            .db
            .insert_inbox_entries(vec![InboxEntry {
                id: format!("{}:{}", alice.id, message.id),
                user_id: alice.id.clone(),
                message_id: message.id.clone(),
                channel_id: message.channel.clone(),
                server_id: Some(other_server.id.clone()),
                kind: InboxKind::Mention,
            }])
            .await
            .unwrap();

        assert!(fetch(&harness, &session.token, "").await.items.is_empty());
    }

    #[rocket::async_test]
    async fn inbox_skips_orphaned_messages_and_channels() {
        let harness = TestHarness::new().await;
        let (_, _, owner) = harness.new_user().await;
        let (server, _) = harness.new_server(&owner).await;
        let channel = harness.new_channel(&server).await;
        let (alice, token, _) = join(&harness, &server, None).await;

        let message = send(&harness, &channel, &owner, &format!("<@{}> valid", alice.id), None).await;
        fetch_until(&harness, &token, "", 1).await;

        // Simulate entries left behind after their message or channel disappeared.
        harness
            .db
            .insert_inbox_entries(vec![
                InboxEntry {
                    id: format!("{}:{}", alice.id, ulid::Ulid::new()),
                    user_id: alice.id.clone(),
                    message_id: ulid::Ulid::new().to_string(),
                    channel_id: message.channel.clone(),
                    server_id: Some(server.id.clone()),
                    kind: InboxKind::Mention,
                },
                InboxEntry {
                    id: format!("{}:{}", alice.id, ulid::Ulid::new()),
                    user_id: alice.id.clone(),
                    message_id: ulid::Ulid::new().to_string(),
                    channel_id: ulid::Ulid::new().to_string(),
                    server_id: Some(server.id.clone()),
                    kind: InboxKind::Mention,
                },
            ])
            .await
            .unwrap();

        let page = fetch(&harness, &token, "").await;
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].message_id, message.id);
    }
}