use redis_kiss::{get_connection as _get_conn, AsyncCommands, Conn};
use regex::Regex;
use revolt_config::config;
use revolt_database::{Channel, Database};
use revolt_models::v0::PushNotification;
use revolt_parser::parse_message;
use revolt_result::{create_error, Result, ToRevoltError};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};
use tokio::join;

async fn get_connection() -> Result<Conn> {
    _get_conn().await.map_err(|_| create_error!(InternalError))
}

pub async fn render_notification_content(
    notification: &PushNotification,
    db: &Database,
) -> Result<String> {
    let parsed = parse_message(&notification.body);

    let user_mentions: HashMap<String, String>;
    let channel_mentions: HashMap<String, String>;
    let emojis: HashMap<String, String>;
    let roles: HashMap<String, String>;

    let server_id: Option<String> = get_channel_server_id(notification.channel.id(), db)
        .await
        .map(Some)
        .or(Ok(None))?;

    if server_id.is_some() {
        (user_mentions, channel_mentions, emojis, roles) = join!(
            get_items(
                parsed.user_mentions,
                server_id.as_deref(),
                db,
                get_user_display_name,
                "Unknown User".to_string()
            ),
            get_items(
                parsed.channel_mentions,
                server_id.as_deref(),
                db,
                get_channel_name,
                "Unknown Channel".to_string()
            ),
            get_items(
                parsed.emojis,
                server_id.as_deref(),
                db,
                get_emoji_name,
                "Unknown Emoji".to_string()
            ),
            get_items(
                parsed.role_mentions,
                server_id.as_deref(),
                db,
                get_role_name,
                "Unknown Role".to_string()
            ),
        );
    } else {
        roles = HashMap::default();

        (user_mentions, channel_mentions, emojis) = join!(
            get_items(
                parsed.user_mentions,
                server_id.as_deref(),
                db,
                get_user_display_name,
                "Unknown User".to_string()
            ),
            get_items(
                parsed.channel_mentions,
                server_id.as_deref(),
                db,
                get_channel_name,
                "Unknown Channel".to_string()
            ),
            get_items(
                parsed.emojis,
                server_id.as_deref(),
                db,
                get_emoji_name,
                "Unknown Emoji".to_string()
            ),
        );
    }

    let mut body = Cow::Borrowed(notification.body.as_str());

    for (uid, name) in user_mentions {
        replace_all_in_place(
            Regex::new(format!("<@{uid}>").as_str()).unwrap(),
            &mut body,
            format!("@{name}").as_str(),
        );
    }

    for (uid, name) in channel_mentions {
        replace_all_in_place(
            Regex::new(format!("<#{uid}>").as_str()).unwrap(),
            &mut body,
            format!("#{name}").as_str(),
        );
    }

    for (uid, name) in roles {
        replace_all_in_place(
            Regex::new(format!("<%{uid}>").as_str()).unwrap(),
            &mut body,
            format!("%{name}").as_str(),
        );
    }

    for (uid, name) in emojis {
        replace_all_in_place(
            Regex::new(format!(":{uid}:").as_str()).unwrap(),
            &mut body,
            format!(":{name}:").as_str(),
        );
    }

    Ok(body.to_string())
}

async fn get_items<F>(
    items: HashSet<String>,
    server_id: Option<&str>,
    db: &Database,
    getter: F,
    invalid_string: String,
) -> HashMap<String, String>
where
    F: AsyncFn(&str, Option<&str>, &Database) -> Result<String>,
{
    let mut resp = HashMap::default();

    for obj_id in items {
        resp.insert(
            obj_id.clone(),
            getter(&obj_id, server_id, db)
                .await
                .unwrap_or(invalid_string.clone()),
        );
    }

    resp
}

// Getters

async fn get_user_display_name(id: &str, server: Option<&str>, db: &Database) -> Result<String> {
    let config = config().await;

    let mut conn = get_connection().await?;
    let key = format!("pushd-user-cache:{}:{id}", server.unwrap_or("GLOBAL"));

    if let Ok(name) = conn.get(key.clone()).await {
        return Ok(name);
    }

    if let Some(server) = server {
        let member = db.fetch_member(server, id).await?;
        if let Some(nickname) = member.nickname {
            conn.set_ex::<_, _, ()>(key, nickname.clone(), config.pushd.render_cache_time)
                .await
                .to_internal_error()?;
            return Ok(nickname);
        }
    }

    let user = db.fetch_user(id).await?;
    let name = user.display_name.unwrap_or(user.username);

    conn.set_ex::<_, _, ()>(key, name.clone(), config.pushd.render_cache_time)
        .await
        .to_internal_error()?;

    Ok(name)
}

async fn get_channel_name(id: &str, _server: Option<&str>, db: &Database) -> Result<String> {
    let config = config().await;

    let mut conn = get_connection().await?;
    let key = format!("pushd-channel-cache:{id}");

    if let Ok(name) = conn.get(key.clone()).await {
        return Ok(name);
    }

    let channel = db.fetch_channel(id).await?;
    let name = match channel {
        Channel::DirectMessage { .. } => "DM Channel".to_string(),
        Channel::Group { name, .. } | Channel::TextChannel { name, .. } => name,
        Channel::SavedMessages { .. } => "Err".to_string(),
    };

    conn.set_ex::<_, _, ()>(key, name.clone(), config.pushd.render_cache_time)
        .await
        .to_internal_error()?;

    Ok(name)
}

async fn get_role_name(id: &str, server: Option<&str>, db: &Database) -> Result<String> {
    let server = server.unwrap(); // Must be passed, but the interface must stay the same as the other getters
    let config: revolt_config::Settings = config().await;

    let mut conn = get_connection().await?;
    let key = format!("pushd-role-cache:{server}:{id}");

    if let Ok(name) = conn.get(key.clone()).await {
        return Ok(name);
    }

    let server = db.fetch_server(server).await?;
    let name = server
        .roles
        .get(id)
        .ok_or_else(|| create_error!(NotFound))?
        .name
        .clone();

    conn.set_ex::<_, _, ()>(key, name.clone(), config.pushd.render_cache_time)
        .await
        .to_internal_error()?;

    Ok(name)
}

async fn get_emoji_name(id: &str, _server: Option<&str>, db: &Database) -> Result<String> {
    let config: revolt_config::Settings = config().await;

    let mut conn = get_connection().await?;
    let key = format!("pushd-emoji-cache:{id}");

    if let Ok(name) = conn.get(key.clone()).await {
        return Ok(name);
    }

    let emoji = db.fetch_emoji(id).await?;
    let name = emoji.name;

    conn.set_ex::<_, _, ()>(key, name.clone(), config.pushd.render_cache_time)
        .await
        .to_internal_error()?;

    Ok(name)
}

// utility

async fn get_channel_server_id(channel_id: &str, db: &Database) -> Result<String> {
    match db.fetch_channel(channel_id).await? {
        Channel::DirectMessage { .. } | Channel::Group { .. } | Channel::SavedMessages { .. } => {
            Err(create_error!(NotFound))
        }
        Channel::TextChannel { server, .. } => Ok(server),
    }
}

fn replace_all_in_place<R: regex::Replacer>(regex: Regex, s: &mut Cow<'_, str>, replacer: R) {
    let new = regex.replace_all(s, replacer);
    if let Cow::Owned(o) = new {
        *s = Cow::Owned(o);
    }
}
