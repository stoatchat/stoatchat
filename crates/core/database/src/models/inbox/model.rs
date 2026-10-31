use std::collections::{HashSet};
use iso8601_timestamp::{Duration, Timestamp};
use revolt_models::v0::MessageFlags;
use crate::{Channel, ChunkedServerMembersGenerator, Database, Member, Message, MessageFlagsValue};
use revolt_result::{ErrorType, Result};
use crate::util::bulk_permissions::BulkDatabasePermissionQuery;

auto_derived!(
    /// Inbox as it is stored in the database
    pub struct InboxEntry {
        /// `{user_id}:{message_id}`, so retries are idempotent
        #[serde(rename = "_id")]
        pub id: String,
        /// The mentioned user ID
        pub user_id: String,
        /// Message ID
        pub message_id: String,
        /// Channel ID
        pub channel_id: String,
        /// Server ID
        #[serde(skip_serializing_if = "Option::is_none")]
        pub server_id: Option<String>,
        /// The inbox kind
        pub kind: InboxKind
    }

    #[derive(Copy, PartialOrd, Ord)]
    pub enum InboxKind {
        Mention,
        Role,
        Everyone,
    }

    pub struct InboxOptions {
        /// The amount of notifications someone can get
        /// Capped out at 100
        pub limit: i64,
        /// Gives inbox items before the given ID
        pub before: Option<String>,
        /// If true mentions get included
        pub mentions: bool,
        /// If true roles get included
        pub roles: bool,
        /// If true everyone mentions get included
        pub everyone: bool,
        /// Only get inbox items for a specific server
        pub server_id: Option<String>,
    }

    /// We send the message for avoiding clients to make constant API calls to fetch messages
    /// they don't have in their cache
    pub struct InboxItem {
        #[serde(rename = "_id")]
        pub id: String,
        pub kind: InboxKind,
        pub message_id: String,
        pub channel_id: String,
        pub server_id: Option<String>,
        pub message: Message,
    }

    /// Pending notification fan-out, persisted with the message so it survives restarts.
    pub struct InboxJob {
        /// Message ID, also used as the job ID
        #[serde(rename = "_id")]
        pub message_id: String,
        /// Channel the message was sent in
        pub channel_id: String,
    }
);

impl InboxEntry {
    fn new(user_id: String, message: &Message, server_id: Option<&str>, kind: InboxKind) -> Self {
        Self {
            id: format!("{user_id}:{}", message.id),
            user_id,
            message_id: message.id.clone(),
            channel_id: message.channel.clone(),
            server_id: server_id.map(str::to_string),
            kind,
        }
    }

    pub async fn create_for_message(
        db: &Database,
        message: &Message,
        channel: &Channel,
    ) -> Result<()> {
        if MessageFlagsValue(message.flags.unwrap_or(0)).has(MessageFlags::SuppressNotifications) {
            return Ok(());
        }

        let server_id = channel.server();
        let chunk_size = revolt_config::config().await.pushd.mass_mention_chunk_size.max(1);
        let sent_at = Timestamp::UNIX_EPOCH + Duration::nanoseconds(message.created_at_ns());

        let mut seen: HashSet<String> = HashSet::new();
        seen.insert(message.author.clone());

        // 1. Direct mentions: already validated and visibility-filtered by create_from_api.
        let direct: Vec<InboxEntry> = message
            .mentions
            .iter()
            .flatten()
            .filter(|user| seen.insert((*user).clone()))
            .map(|user| Self::new(user.clone(), message, server_id, InboxKind::Mention))
            .collect();
        if !direct.is_empty() {
            db.insert_inbox_entries(direct).await?;
        }

        let Some(server_id) = server_id else {
            return Ok(());
        };

        // 2. Role mentions
        if let Some(roles) = message.role_mentions.as_ref().filter(|r| !r.is_empty()) {
            let mut members = db.fetch_all_members_with_roles_chunked(server_id, roles).await?;
            Self::fan_out(db, message, channel, server_id, InboxKind::Role, Some(roles), sent_at, &mut members, &mut seen, chunk_size).await?;
        }

        // 3. @everyone (lowest priority)
        if MessageFlagsValue(message.flags.unwrap_or(0)).has(MessageFlags::MentionsEveryone) {
            let mut members = db.fetch_all_members_chunked(server_id).await?;
            Self::fan_out(db, message, channel, server_id, InboxKind::Everyone, None, sent_at, &mut members, &mut seen, chunk_size).await?;
        }

        Ok(())
    }

    /// Whether this member was eligible for the mention when the message was sent.
    fn eligible_at(member: &Member, sent_at: Timestamp, roles: Option<&[String]>) -> bool {
        if member.joined_at > sent_at {
            return false;
        }

        roles.is_none_or(|roles| {
            roles.iter().any(|role| {
                member
                    .role_since(role)
                    .is_some_and(|granted_at| granted_at <= sent_at)
            })
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn fan_out(
        db: &Database,
        message: &Message,
        channel: &Channel,
        server_id: &str,
        kind: InboxKind,
        roles: Option<&[String]>,
        sent_at: Timestamp,
        members: &mut ChunkedServerMembersGenerator,
        seen: &mut HashSet<String>,
        chunk_size: usize,
    ) -> Result<()> {
        let mut chunk: Vec<Member> = Vec::with_capacity(chunk_size);

        while let Some(member) = members.next().await? {
            if !Self::eligible_at(&member, sent_at, roles) {
                // Not eligible for this mention kind. Do not add to `seen`, since
                // they may still be eligible for a lower-priority @everyone.
                continue;
            }

            // Mark as seen even if they turn out not to see the channel:
            // a lower-priority kind can't make them visible either.
            if seen.insert(member.id.user.clone()) {
                chunk.push(member);
                if chunk.len() >= chunk_size {
                    Self::store_visible(db, message, channel, server_id, kind, std::mem::take(&mut chunk)).await?;
                }
            }
        }

        if !chunk.is_empty() {
            Self::store_visible(db, message, channel, server_id, kind, chunk).await?;
        }
        Ok(())
    }

    async fn store_visible(
        db: &Database,
        message: &Message,
        channel: &Channel,
        server_id: &str,
        kind: InboxKind,
        members: Vec<Member>,
    ) -> Result<()> {
        let visible = BulkDatabasePermissionQuery::from_server_id(db, server_id)
            .await?
            .channel(channel)
            .members(&members)
            .members_can_see_channel()
            .await?;

        let entries: Vec<InboxEntry> = members
            .into_iter()
            .filter(|m| *visible.get(&m.id.user).unwrap_or(&false))
            .map(|m| Self::new(m.id.user, message, Some(server_id), kind))
            .collect();

        if entries.is_empty() {
            return Ok(());
        }
        db.insert_inbox_entries(entries).await
    }
}

impl InboxJob {
    /// Process the fan-out job. The job stays persisted on transient failures so a
    /// background worker can retry it; missing messages/channels are discarded.
    pub async fn process(&self, db: &Database) -> Result<()> {
        let channel = match db.fetch_channel(&self.channel_id).await {
            Ok(channel) => channel,
            Err(e) if matches!(e.error_type, ErrorType::NotFound) => {
                return db.delete_inbox_job(&self.message_id).await;
            }
            Err(e) => return Err(e),
        };

        let message = match db.fetch_message(&self.message_id).await {
            Ok(message) => message,
            Err(e) if matches!(e.error_type, ErrorType::NotFound) => {
                return db.delete_inbox_job(&self.message_id).await;
            }
            Err(e) => return Err(e),
        };

        InboxEntry::create_for_message(db, &message, &channel).await?;
        db.delete_inbox_job(&self.message_id).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use iso8601_timestamp::{Duration as IsoDuration, Timestamp};
    use ulid::Ulid;

    use crate::{fixture, Channel, Database, InboxEntry, InboxJob, InboxKind, InboxOptions, Member, MemberCompositeKey, Message, MessageFlagsValue, User};
    use revolt_models::v0::MessageFlags;
    use lapin::{options::ExchangeDeclareOptions, types::FieldTable, ExchangeKind};

    async fn kind_of(db: &Database, user: &str) -> InboxKind {
        let got = db.fetch_inbox_entries(user, &opts(50)).await.unwrap();
        assert_eq!(got.len(), 1, "{user}");
        got[0].kind
    }

    /// Deterministic, time-ordered message id: higher `n` = newer.
    fn mid(n: u64) -> String {
        Ulid::from_parts(n, 0).to_string()
    }

    fn ns(timestamp: Timestamp) -> i64 {
        timestamp
            .duration_since(Timestamp::UNIX_EPOCH)
            .whole_nanoseconds() as i64
    }

    fn entry(user: &str, n: u64, channel: &str, server: Option<&str>, kind: InboxKind) -> InboxEntry {
        let message_id = mid(n);
        InboxEntry {
            id: format!("{user}:{message_id}"),
            user_id: user.to_string(),
            message_id,
            channel_id: channel.to_string(),
            server_id: server.map(str::to_string),
            kind,
        }
    }

    fn opts(limit: i64) -> InboxOptions {
        InboxOptions {
            limit,
            before: None,
            mentions: true,
            roles: true,
            everyone: true,
            server_id: None,
        }
    }

    fn ids(entries: &[InboxEntry]) -> Vec<String> {
        entries.iter().map(|e| e.message_id.clone()).collect()
    }

    #[tokio::test]
    async fn insert_is_idempotent() {
        database_test!(|db| async move {
            let e = entry("u1", 1, "c1", Some("s1"), InboxKind::Mention);
            db.insert_inbox_entries(vec![e.clone()]).await.unwrap();
            // A retried send re-inserts the same deterministic _id.
            db.insert_inbox_entries(vec![e.clone(), entry("u1", 2, "c1", Some("s1"), InboxKind::Mention)])
                .await
                .unwrap();

            let all = db.fetch_inbox_entries("u1", &opts(50)).await.unwrap();
            assert_eq!(all.len(), 2);
        });
    }

    #[tokio::test]
    async fn fetch_is_newest_first_and_scoped_to_user() {
        database_test!(|db| async move {
            db.insert_inbox_entries(vec![
                entry("u1", 1, "c1", None, InboxKind::Mention),
                entry("u1", 3, "c1", None, InboxKind::Mention),
                entry("u1", 2, "c1", None, InboxKind::Mention),
                entry("u2", 4, "c1", None, InboxKind::Mention),
            ])
            .await
            .unwrap();

            let got = db.fetch_inbox_entries("u1", &opts(50)).await.unwrap();
            assert_eq!(ids(&got), vec![mid(3), mid(2), mid(1)]);
        });
    }

    #[tokio::test]
    async fn fetch_applies_kind_and_server_filters() {
        database_test!(|db| async move {
            db.insert_inbox_entries(vec![
                entry("u1", 1, "c1", Some("s1"), InboxKind::Mention),
                entry("u1", 2, "c1", Some("s1"), InboxKind::Role),
                entry("u1", 3, "c2", Some("s2"), InboxKind::Everyone),
                entry("u1", 4, "dm", None, InboxKind::Mention),
            ])
            .await
            .unwrap();

            let mut o = opts(50);
            o.roles = false;
            o.everyone = false;
            let got = db.fetch_inbox_entries("u1", &o).await.unwrap();
            assert_eq!(ids(&got), vec![mid(4), mid(1)]);

            let mut o = opts(50);
            o.server_id = Some("s2".into());
            let got = db.fetch_inbox_entries("u1", &o).await.unwrap();
            assert_eq!(ids(&got), vec![mid(3)]);

            let mut o = opts(50);
            (o.mentions, o.roles, o.everyone) = (false, false, false);
            assert!(db.fetch_inbox_entries("u1", &o).await.unwrap().is_empty());
        });
    }

    #[tokio::test]
    async fn paging_with_before_matches_a_single_fetch() {
        database_test!(|db| async move {
            let entries: Vec<_> = (1..=10)
                .map(|n| entry("u1", n, "c1", None, InboxKind::Mention))
                .collect();
            db.insert_inbox_entries(entries).await.unwrap();

            let full = ids(&db.fetch_inbox_entries("u1", &opts(100)).await.unwrap());

            for limit in [1, 2, 3, 7, 10] {
                let mut paged = vec![];
                let mut o = opts(limit);
                loop {
                    let page = db.fetch_inbox_entries("u1", &o).await.unwrap();
                    let Some(last) = page.last() else { break };
                    o.before = Some(last.message_id.clone());
                    paged.extend(ids(&page));
                }
                assert_eq!(paged, full, "limit {limit}");
            }
        });
    }

    #[tokio::test]
    async fn delete_for_messages_channel_server_and_user() {
        database_test!(|db| async move {
            db.insert_inbox_entries(vec![
                entry("u1", 1, "c1", Some("s1"), InboxKind::Mention),
                entry("u1", 2, "c2", Some("s1"), InboxKind::Mention),
                entry("u1", 3, "c3", Some("s2"), InboxKind::Mention),
                entry("u1", 4, "dm", None, InboxKind::Mention),
                entry("u2", 1, "c1", Some("s1"), InboxKind::Mention),
            ])
            .await
            .unwrap();

            // messages: removed for every user
            db.delete_inbox_entries_for_messages(&[mid(1)]).await.unwrap();
            assert!(!ids(&db.fetch_inbox_entries("u1", &opts(50)).await.unwrap()).contains(&mid(1)));
            assert!(db.fetch_inbox_entries("u2", &opts(50)).await.unwrap().is_empty());

            // channel
            db.delete_inbox_entries_for_channel("c2").await.unwrap();
            assert_eq!(
                ids(&db.fetch_inbox_entries("u1", &opts(50)).await.unwrap()),
                vec![mid(4), mid(3)]
            );

            // server: DM entries (no server) are untouched
            db.delete_inbox_entries_for_server("s2").await.unwrap();
            assert_eq!(
                ids(&db.fetch_inbox_entries("u1", &opts(50)).await.unwrap()),
                vec![mid(4)]
            );
        });
    }

    #[tokio::test]
    async fn delete_for_user_respects_server_scope() {
        database_test!(|db| async move {
            db.insert_inbox_entries(vec![
                entry("u1", 1, "c1", Some("s1"), InboxKind::Mention),
                entry("u1", 2, "c2", Some("s2"), InboxKind::Mention),
                entry("u2", 3, "c1", Some("s1"), InboxKind::Mention),
            ])
            .await
            .unwrap();

            // Left s1: only their s1 entries go, other users and servers stay.
            db.delete_inbox_entries_for_user("u1", Some("s1")).await.unwrap();
            assert_eq!(ids(&db.fetch_inbox_entries("u1", &opts(50)).await.unwrap()), vec![mid(2)]);
            assert_eq!(db.fetch_inbox_entries("u2", &opts(50)).await.unwrap().len(), 1);

            // Account removed: everything for that user.
            db.delete_inbox_entries_for_user("u1", None).await.unwrap();
            assert!(db.fetch_inbox_entries("u1", &opts(50)).await.unwrap().is_empty());
            assert_eq!(db.fetch_inbox_entries("u2", &opts(50)).await.unwrap().len(), 1);
        });
    }

    #[tokio::test]
    async fn prune_removes_only_entries_older_than_cutoff() {
        database_test!(|db| async move {
            let day = Duration::from_secs(24 * 60 * 60);
            let now = SystemTime::now();
            let id_at = |t: SystemTime| Ulid::from_datetime(t).to_string();

            let mut old = entry("u1", 0, "c1", None, InboxKind::Mention);
            old.message_id = id_at(now - 40 * day);
            old.id = format!("u1:{}", old.message_id);

            let mut fresh = entry("u1", 0, "c1", None, InboxKind::Mention);
            fresh.message_id = id_at(now - 5 * day);
            fresh.id = format!("u1:{}", fresh.message_id);

            db.insert_inbox_entries(vec![old, fresh.clone()]).await.unwrap();

            let removed = db.prune_inbox_entries(now - 30 * day).await.unwrap();
            assert_eq!(removed, 1);

            let left = db.fetch_inbox_entries("u1", &opts(50)).await.unwrap();
            assert_eq!(ids(&left), vec![fresh.message_id]);

            // Nothing left to prune.
            assert_eq!(db.prune_inbox_entries(now - 30 * day).await.unwrap(), 0);
        });
    }

    #[tokio::test]
    async fn create_for_message_dedupes_and_skips_author() {
        database_test!(|db| async move {
            let channel = Channel::DirectMessage {
                id: "dm".into(),
                active: true,
                recipients: vec!["author".into(), "a".into(), "b".into()],
                last_message_id: None,
            };
            let message = Message {
                id: mid(1),
                channel: "dm".into(),
                author: "author".into(),
                // author mentioned themselves + duplicate user
                mentions: Some(vec!["a".into(), "b".into(), "a".into(), "author".into()]),
                ..Default::default()
            };

            InboxEntry::create_for_message(&db, &message, &channel).await.unwrap();
            // Retry must not duplicate.
            InboxEntry::create_for_message(&db, &message, &channel).await.unwrap();

            for user in ["a", "b"] {
                let got = db.fetch_inbox_entries(user, &opts(50)).await.unwrap();
                assert_eq!(got.len(), 1, "{user}");
                assert_eq!(got[0].kind, InboxKind::Mention);
            }
            assert!(db.fetch_inbox_entries("author", &opts(50)).await.unwrap().is_empty());
        });
    }
    #[tokio::test]
    async fn create_for_message_priority_and_fanout() {
        database_test!(|db| async move {
        fixture!(db, "server_with_roles",
            owner user 0
            moderator user 1
            plain user 2
            channel channel 3
            server server 4);

        let role = |name: &str| {
            server.roles.values().find(|role| role.name == name).unwrap().id.clone()
        };
        let owner_role = role("Owner");
        let moderator_role = role("Moderator");

        let new_user = |id: &str| User {
            id: id.into(),
            username: id.into(),
            discriminator: "0001".into(),
            ..Default::default()
        };
        let new_member = |user: &str, roles: Vec<&str>| Member {
            id: MemberCompositeKey { server: server.id.clone(), user: user.into() },
            roles: roles.into_iter().map(String::from).collect(),
            ..Default::default()
        };

        for user_id in ["visible_owner", "visible_plain", "hidden_owner"] {
            db.insert_user(&new_user(user_id)).await.unwrap();
        }
        // The Owner role has no ViewChannel override here, while Moderator does.
        db.insert_or_merge_member(&new_member("visible_owner", vec![&owner_role, &moderator_role])).await.unwrap();
        db.insert_or_merge_member(&new_member("visible_plain", vec![&moderator_role])).await.unwrap();
        db.insert_or_merge_member(&new_member("hidden_owner", vec![&owner_role])).await.unwrap();

        let mut flags = MessageFlagsValue(0);
        flags.set(MessageFlags::MentionsEveryone, true);

        let message = Message {
            id: mid(1),
            channel: channel.id().to_string(),
            author: owner.id.clone(),
            mentions: Some(vec![moderator.id.clone()]),
            role_mentions: Some(vec![owner_role]),
            flags: Some(flags.0),
            created_at_ns: Some(ns(Timestamp::now_utc())),
            ..Default::default()
        };
        InboxEntry::create_for_message(&db, &message, &channel).await.unwrap();

        assert_eq!(kind_of(&db, &moderator.id).await, InboxKind::Mention);
        assert_eq!(kind_of(&db, "visible_owner").await, InboxKind::Role);
        assert_eq!(kind_of(&db, "visible_plain").await, InboxKind::Everyone);
        // Mentioned role and @everyone both skip members who cannot view the channel.
        assert!(db.fetch_inbox_entries("hidden_owner", &opts(50)).await.unwrap().is_empty());
        assert!(db.fetch_inbox_entries(&plain.id, &opts(50)).await.unwrap().is_empty());
        assert!(db.fetch_inbox_entries(&owner.id, &opts(50)).await.unwrap().is_empty());
        });
    }

    #[tokio::test]
    async fn create_for_message_uses_send_time_eligibility() {
        database_test!(|db| async move {
            fixture!(db, "server_with_roles",
                owner user 0
                channel channel 3
                server server 4);

            let owner_role = server.roles.values().find(|role| role.name == "Owner").unwrap().id.clone();
            let moderator_role = server.roles.values().find(|role| role.name == "Moderator").unwrap().id.clone();
            let sent_at = Timestamp::now_utc();
            let before = sent_at - IsoDuration::hours(1);
            let after = sent_at + IsoDuration::hours(1);

            let new_user = |id: &str| User {
                id: id.into(),
                username: id.into(),
                discriminator: "0001".into(),
                ..Default::default()
            };
            for user_id in ["early_role", "late_role", "late_join"] {
                db.insert_user(&new_user(user_id)).await.unwrap();
            }

            let member = |user: &str, joined_at, roles: Vec<String>, role_since| Member {
                id: MemberCompositeKey { server: server.id.clone(), user: user.into() },
                joined_at,
                roles,
                role_since,
                ..Default::default()
            };

            // Moderator grants ViewChannel, so visibility does not confound the
            // membership/role-grant time assertions below.
            db.insert_or_merge_member(&member(
                "early_role",
                before,
                vec![owner_role.clone(), moderator_role.clone()],
                [
                    (owner_role.clone(), before),
                    (moderator_role.clone(), before),
                ]
                .into(),
            )).await.unwrap();
            // Held the role at processing time, but not when the message was sent.
            db.insert_or_merge_member(&member(
                "late_role",
                before,
                vec![owner_role.clone(), moderator_role.clone()],
                [
                    (owner_role.clone(), after),
                    (moderator_role.clone(), before),
                ]
                .into(),
            )).await.unwrap();
            // Joined after the message was sent.
            db.insert_or_merge_member(&member(
                "late_join",
                after,
                vec![moderator_role.clone()],
                [(moderator_role, before)].into(),
            )).await.unwrap();

            let mut flags = MessageFlagsValue(0);
            flags.set(MessageFlags::MentionsEveryone, true);

            let message = Message {
                id: mid(1),
                channel: channel.id().to_string(),
                author: owner.id.clone(),
                role_mentions: Some(vec![owner_role]),
                flags: Some(flags.0),
                created_at_ns: Some(ns(sent_at)),
                ..Default::default()
            };
            InboxEntry::create_for_message(&db, &message, &channel).await.unwrap();

            assert_eq!(kind_of(&db, "early_role").await, InboxKind::Role);
            // A late role grant must not block a valid @everyone notification.
            assert_eq!(kind_of(&db, "late_role").await, InboxKind::Everyone);
            assert!(db.fetch_inbox_entries("late_join", &opts(50)).await.unwrap().is_empty());
        });
    }

    #[tokio::test]
    async fn inbox_job_survives_until_processed() {
        database_test!(|db| async move {
            fixture!(db, "server_with_roles",
                owner user 0
                moderator user 1
                channel channel 3
                _server server 4);

            let message = Message {
                id: mid(1),
                channel: channel.id().to_string(),
                author: owner.id.clone(),
                mentions: Some(vec![moderator.id.clone()]),
                created_at_ns: Some(ns(Timestamp::now_utc())),
                ..Default::default()
            };
            db.insert_message(&message).await.unwrap();

            let job = InboxJob {
                message_id: message.id.clone(),
                channel_id: message.channel.clone(),
            };
            db.insert_inbox_job(&job).await.unwrap();
            // Re-queueing the same message is idempotent.
            db.insert_inbox_job(&job).await.unwrap();
            assert_eq!(db.fetch_inbox_jobs(10).await.unwrap().len(), 1);

            job.process(&db).await.unwrap();
            assert!(db.fetch_inbox_jobs(10).await.unwrap().is_empty());
            assert_eq!(kind_of(&db, &moderator.id).await, InboxKind::Mention);

            // Retried fan-out stays idempotent.
            job.process(&db).await.unwrap();
            assert_eq!(kind_of(&db, &moderator.id).await, InboxKind::Mention);

            // Jobs whose message disappeared are discarded instead of retried forever.
            let missing = InboxJob {
                message_id: mid(2),
                channel_id: message.channel.clone(),
            };
            db.insert_inbox_job(&missing).await.unwrap();
            missing.process(&db).await.unwrap();
            assert!(db.fetch_inbox_jobs(10).await.unwrap().is_empty());
        });
    }
}