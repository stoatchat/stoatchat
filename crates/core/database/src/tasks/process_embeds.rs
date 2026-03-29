use crate::{AMQP, AppendMessage, Database, models::Message};

use futures::future::join_all;
use linkify::{LinkFinder, LinkKind};
use regex::Regex;
use revolt_config::config;
use revolt_result::Result;

use async_lock::Semaphore;
use async_std::task::spawn;
use deadqueue::limited::Queue;
use once_cell::sync::Lazy;
use revolt_models::v0::Embed;
use std::{collections::HashSet, sync::Arc};

use isahc::prelude::*;

/// Task information
#[derive(Debug)]
struct EmbedTask {
    /// Channel we're processing the event in
    channel: String,
    /// ID of the message we're processing
    id: String,
    /// Content of the message
    content: String,
}

static Q: Lazy<Queue<EmbedTask>> = Lazy::new(|| Queue::new(10_000));

/// Queue a new task for a worker
pub async fn queue(channel: String, id: String, content: String) {
    Q.try_push(EmbedTask {
        channel,
        id,
        content,
    })
    .ok();

    info!("Queue is using {} slots from {}.", Q.len(), Q.capacity());
}

/// Start a new worker
pub async fn worker(db: Database, amqp: AMQP) {
    let semaphore = Arc::new(Semaphore::new(
        config().await.api.workers.max_concurrent_connections,
    ));

    loop {
        let task = Q.pop().await;
        let db = db.clone();
        let amqp = amqp.clone();
        let semaphore = semaphore.clone();

        spawn(async move {
            let config = config().await;
            let embeds = generate(
                task.content,
                &config.hosts.january,
                config.features.limits.global.message_embeds,
                semaphore,
            )
            .await;

            if let Ok(embeds) = embeds {
                if let Err(err) = Message::append(
                    &db,
                    Some(&amqp),
                    task.id,
                    task.channel,
                    AppendMessage {
                        embeds: Some(embeds),
                    },
                )
                .await
                {
                    error!("Encountered an error appending to message: {:?}", err);
                }
            }
        });
    }
}

static RE_CODE: Lazy<Regex> = Lazy::new(|| Regex::new("```(?:.|\n)+?```|`(?:.|\n)+?`").unwrap());
static RE_IGNORED: Lazy<Regex> = Lazy::new(|| Regex::new("(<http.+>)").unwrap());

pub async fn generate(
    content: String,
    host: &str,
    max_embeds: usize,
    semaphore: Arc<Semaphore>,
) -> Result<Vec<Embed>> {
    // Ignore code blocks.
    let content = RE_CODE.replace_all(&content, "");

    // Ignore all content between angle brackets starting with http.
    let content = RE_IGNORED.replace_all(&content, "");

    let content = content
        // Ignore quoted lines.
        .split('\n')
        .map(|v| {
            if let Some(c) = v.chars().next() {
                if c == '>' {
                    return "";
                }
            }

            v
        })
        .collect::<Vec<&str>>()
        .join("\n");

    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Url]);

    // Process all links, stripping anchors and
    // only taking up to `max_embeds` of links.
    let links: Vec<String> = finder
        .links(&content)
        .map(|x| {
            x.as_str()
                .chars()
                .take_while(|&ch| ch != '#')
                .collect::<String>()
        })
        .collect::<HashSet<String>>()
        .into_iter()
        .take(max_embeds)
        .collect();

    // If no links, fail out.
    if links.is_empty() {
        return Err(create_error!(LabelMe));
    }

    // TODO: batch request to january
    let mut tasks = Vec::new();

    for link in links {
        let semaphore = semaphore.clone();
        let host = host.to_string();
        tasks.push(spawn(async move {
            let guard = semaphore.acquire().await;

            if let Ok(mut response) = isahc::get_async(format!(
                "{host}/embed?url={}",
                url_escape::encode_component(&link)
            ))
            .await
            {
                drop(guard);
                response.json::<Embed>().await.ok()
            } else {
                None
            }
        }));
    }

    let embeds = join_all(tasks)
        .await
        .into_iter()
        .flatten()
        .collect::<Vec<Embed>>();

    // Prevent database update when no embeds are found.
    if !embeds.is_empty() {
        Ok(embeds)
    } else {
        Err(create_error!(LabelMe))
    }
}
