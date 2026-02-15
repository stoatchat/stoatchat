use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;
use revolt_models::v0::{
    BandcampType, Image, ImageSize, LightspeedType, Special, TwitchType, Video, WebsiteMetadata,
};
use scraper::{Html, Selector};

/// Create website metadata from URL and document
pub async fn create_website_embed(original_url: &str, document: &str) -> Option<WebsiteMetadata> {
    let (mut meta, mut link) = {
        let document = Html::parse_document(document);

        // create selectors
        let meta_selector = Selector::parse("meta").ok()?;
        let link_selector = Selector::parse("link").ok()?;

        // extract meta tags
        let mut meta = HashMap::new();
        for el in document.select(&meta_selector) {
            let node = el.value();

            if let (Some(property), Some(content)) = (
                node.attr("property").or_else(|| node.attr("name")),
                node.attr("content"),
            ) {
                meta.insert(property.to_string(), content.to_string());
            }
        }

        // extract rel links
        let mut link = HashMap::new();
        for el in document.select(&link_selector) {
            let node = el.value();

            if let (Some(property), Some(content)) = (node.attr("rel"), node.attr("href")) {
                link.insert(property.to_string(), content.to_string());
            }
        }

        (meta, link)
    };

    // build metadata
    let mut metadata = WebsiteMetadata {
        title: meta
            .remove("og:title")
            .or_else(|| meta.remove("twitter:title"))
            .or_else(|| meta.remove("title"))
            .map(|s| s.trim().to_owned()),
        description: meta
            .remove("og:description")
            .or_else(|| meta.remove("twitter:description"))
            .or_else(|| meta.remove("description"))
            .map(|s| s.trim().to_owned()),
        image: meta
            .remove("og:image")
            .or_else(|| meta.remove("og:image:secure_url"))
            .or_else(|| meta.remove("twitter:image"))
            .or_else(|| meta.remove("twitter:image:src"))
            .map(|s| s.trim().to_owned())
            .map(|mut url| {
                // If relative URL, prepend root URL. Also if root URL ends with a slash, remove it.
                if let Some(ch) = url.chars().next() {
                    if ch == '/' {
                        url = format!("{}{}", &original_url.trim_end_matches('/'), url);
                    }
                }

                let mut size = ImageSize::Preview;
                if let Some(card) = meta.remove("twitter:card") {
                    if &card == "summary_large_image" {
                        size = ImageSize::Large;
                    }
                }

                Image {
                    url: url.to_owned(),
                    width: meta
                        .remove("og:image:width")
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or(0),
                    height: meta
                        .remove("og:image:height")
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or(0),
                    size,
                }
            }),
        video: meta
            .remove("og:video")
            .or_else(|| meta.remove("og:video:url"))
            .or_else(|| meta.remove("og:video:secure_url"))
            .map(|s| s.trim().to_owned())
            .map(|mut url| {
                // If relative URL, prepend root URL. Also if root URL ends with a slash, remove it.
                if let Some(ch) = url.chars().next() {
                    if ch == '/' {
                        url = format!("{}{}", &original_url.trim_end_matches('/'), url);
                    }
                }

                Video {
                    url: url.to_owned(),
                    width: meta
                        .remove("og:video:width")
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or(0),
                    height: meta
                        .remove("og:video:height")
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or(0),
                }
            }),
        icon_url: link
            .remove("apple-touch-icon")
            .or_else(|| link.remove("icon"))
            .map(|s| s.trim().to_owned())
            .map(|mut v| {
                // If relative URL, prepend root URL.
                if let Some(ch) = v.chars().next() {
                    if ch == '/' {
                        v = format!("{}{}", &original_url.trim_end_matches('/'), v);
                    }
                }

                v
            }),
        colour: meta.remove("theme-color").map(|s| s.trim().to_owned()),
        site_name: meta.remove("og:site_name").map(|s| s.trim().to_owned()),
        url: meta
            .remove("og:url")
            .or_else(|| Some(original_url.to_owned())),
        original_url: Some(original_url.to_owned()),
        special: None,
    };

    // populate extra metadata for popular websites
    populate_special(original_url.to_owned(), &mut metadata).await;

    // fetch video size if missing
    if metadata.special.is_none() {
        if let Some(Video { width, height, url }) = &metadata.video {
            if width == &0 || height == &0 {
                metadata.video =
                    match crate::requests::Request::fetch_video_metadata(url, None).await {
                        Ok(Some(video)) => Some(video),
                        _ => None,
                    }
            }
        }
    }

    // remove image if video exists
    if metadata.video.is_some() {
        metadata.image.take();
    }

    // fetch image size if missing
    if metadata.special.is_none() {
        if let Some(Image {
            width, height, url, ..
        }) = &metadata.image
        {
            if width == &0 || height == &0 {
                metadata.image =
                    match crate::requests::Request::fetch_image_metadata(url, None).await {
                        Ok(Some(image)) => Some(image),
                        _ => None,
                    }
            }
        }
    }

    // truncate data
    metadata.truncate();

    // if it's empty, don't return anything
    if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    }
}

pub async fn populate_special(original_url: String, metadata: &mut WebsiteMetadata) {
    lazy_static! {
        static ref RE_YOUTUBE: Regex = Regex::new("^(?:(?:https?:)?//)?(?:(?:www|m)\\.)?(?:(?:youtube\\.com|youtu.be))(?:/(?:[\\w\\-]+\\?v=|embed/|v/)?)([\\w\\-]+)(?:\\S+)?$").unwrap();

        static ref RE_LIGHTSPEED: Regex = Regex::new("^(?:https?://)?(?:[\\w]+\\.)?lightspeed\\.tv/([a-z0-9_]{4,25})").unwrap();

        static ref RE_TWITCH: Regex = Regex::new("^(?:https?://)?(?:www\\.|go\\.)?twitch\\.tv/([a-z0-9_]+)($|\\?)").unwrap();
        static ref RE_TWITCH_VOD: Regex = Regex::new("^(?:https?://)?(?:www\\.|go\\.)?twitch\\.tv/videos/([0-9]+)($|\\?)").unwrap();
        static ref RE_TWITCH_CLIP: Regex = Regex::new("^(?:https?://)?(?:www\\.|go\\.)?twitch\\.tv/(?:[a-z0-9_]+)/clip/([A-z0-9_-]+)($|\\?)").unwrap();

        static ref RE_SPOTIFY: Regex = Regex::new("^(?:https?://)?open.spotify.com/(track|user|artist|album|playlist)/([A-z0-9]+)").unwrap();
        static ref RE_SOUNDCLOUD: Regex = Regex::new("^(?:https?://)?soundcloud.com/([a-zA-Z0-9-]+)/([A-z0-9-]+)").unwrap();
        static ref RE_BANDCAMP: Regex = Regex::new("^(?:https?://)?(?:[A-z0-9_-]+).bandcamp.com/(track|album)/([A-z0-9_-]+)").unwrap();
        static ref RE_APPLE_MUSIC: Regex = Regex::new("^(?:https?://)?music\\.apple\\.com/(?:[a-z]{2}/)?album/(?:[a-zA-Z0-9-]+)/(\\d+)(?:\\?i=(\\d+))?").unwrap();

        static ref RE_STREAMABLE: Regex = Regex::new("^(?:https?://)?(?:www\\.)?streamable\\.com/([\\w\\d-]+)").unwrap();

        static ref RE_GIF: Regex = Regex::new("^(?:https?://)?(www\\.)?(gifbox\\.me/view|yiffbox\\.me/view|tenor\\.com/view|giphy\\.com/gifs|klipy\\.com/gifs|gfycat\\.com|redgifs\\.com/watch)/[\\w\\d-]+").unwrap();
    }

    let url = metadata
        .url
        .as_ref()
        .or(metadata.original_url.as_ref())
        .unwrap_or(&original_url)
        .as_str();

    metadata.special = if let Some(captures) = RE_STREAMABLE.captures_iter(url).next() {
        Some(Special::Streamable {
            id: captures[1].to_string(),
        })
    } else if let Some(captures) = RE_YOUTUBE.captures_iter(url).next() {
        let id = captures[1].to_string();

        lazy_static! {
            static ref RE_TIMESTAMP: Regex = Regex::new("(?:\\?|&)(?:t|start)=([\\w]+)").unwrap();
        }

        // YouTube now blocks datacentre IPs from fetching information
        // This is a fallback to prevent the embed from looking weird
        if metadata.video.is_none() {
            metadata.title.replace("YouTube".to_owned());
            metadata.description.take();
            metadata.colour.take();
            metadata.icon_url.take();
            metadata.site_name.take();

            // Verify the video exists
            if !crate::requests::Request::exists(&format!(
                "http://img.youtube.com/vi/{}/sddefault.jpg",
                id
            ))
            .await
            {
                return;
            }
        }

        if let Some(timestamp_captures) = RE_TIMESTAMP.captures_iter(url).next() {
            Some(Special::YouTube {
                id,
                timestamp: Some(timestamp_captures[1].to_string()),
            })
        } else {
            Some(Special::YouTube {
                id,
                timestamp: None,
            })
        }
    } else if let Some(captures) = RE_LIGHTSPEED.captures_iter(url).next() {
        Some(Special::Lightspeed {
            id: captures[1].to_string(),
            content_type: LightspeedType::Channel,
        })
    } else if let Some(captures) = RE_TWITCH.captures_iter(url).next() {
        Some(Special::Twitch {
            id: captures[1].to_string(),
            content_type: TwitchType::Channel,
        })
    } else if let Some(captures) = RE_TWITCH_VOD.captures_iter(url).next() {
        Some(Special::Twitch {
            id: captures[1].to_string(),
            content_type: TwitchType::Video,
        })
    } else if let Some(captures) = RE_TWITCH_CLIP.captures_iter(url).next() {
        Some(Special::Twitch {
            id: captures[1].to_string(),
            content_type: TwitchType::Clip,
        })
    } else if let Some(captures) = RE_SPOTIFY.captures_iter(url).next() {
        Some(Special::Spotify {
            content_type: captures[1].to_string(),
            id: captures[2].to_string(),
        })
    } else if RE_SOUNDCLOUD.is_match(url) {
        Some(Special::Soundcloud)
    } else if RE_BANDCAMP.is_match(url) {
        lazy_static! {
            static ref RE_TRACK: Regex = Regex::new("track=(\\d+)").unwrap();
            static ref RE_ALBUM: Regex = Regex::new("album=(\\d+)").unwrap();
        }

        if let Some(video) = &metadata.video {
            if let Some(captures) = RE_TRACK.captures_iter(&video.url).next() {
                Some(Special::Bandcamp {
                    content_type: BandcampType::Track,
                    id: captures[1].to_string(),
                })
            } else {
                RE_ALBUM
                    .captures_iter(&video.url)
                    .next()
                    .map(|captures| Special::Bandcamp {
                        content_type: BandcampType::Album,
                        id: captures[1].to_string(),
                    })
            }
        } else {
            None
        }
    } else if RE_GIF.is_match(url) {
        Some(Special::GIF)
    } else {
        RE_APPLE_MUSIC
            .captures_iter(&original_url)
            .next()
            .map(|captures| Special::AppleMusic {
                album_id: captures[1].to_string(),
                track_id: captures.get(2).map(|m| m.as_str().to_string()),
            })
    };

    // add colours for popular websites
    if let Some(special) = &metadata.special {
        match special {
            Special::YouTube { .. } => metadata.colour = Some("#FF424F".to_string()),
            Special::Twitch { .. } => metadata.colour = Some("#7B68EE".to_string()),
            Special::Lightspeed { .. } => metadata.colour = Some("#7445D9".to_string()),
            Special::Spotify { .. } => metadata.colour = Some("#1ABC9C".to_string()),
            Special::Soundcloud => metadata.colour = Some("#FF7F50".to_string()),
            Special::AppleMusic { .. } => metadata.colour = Some("#FA233B".to_string()),
            _ => {}
        }
    }
}
