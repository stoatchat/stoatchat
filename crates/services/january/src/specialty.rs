use crate::site_models::{fxtwitter::FxTwitterResult, youtube::YoutubeOEmbed};
use revolt_models::v0::{Embed, Image, Special, Video, WebsiteMetadata};
use revolt_result::{Result, create_error};

use crate::requests::{RE_URL_YOUTUBE, Request};
use crate::site_models::fxtwitter::{FxTwitterMediaElement, FxTwitterQuote};
pub struct SpecialtySitesGenerator {}

impl SpecialtySitesGenerator {
    pub async fn youtube(url: &str, request: Request) -> Result<Embed> {
        let json: YoutubeOEmbed = request
            .response
            .json()
            .await
            .map_err(|_| create_error!(ProxyError))?;

        let captures = RE_URL_YOUTUBE
            .captures(url)
            .ok_or_else(|| create_error!(ProxyError))?;
        let id = captures[1].to_string();
        let timestamp = captures
            .get(2)
            .map(|e| Some(e.as_str().to_string()))
            .unwrap_or(None);

        Ok(Embed::Website(WebsiteMetadata {
            url: Some(url.to_string()),
            original_url: None,
            special: Some(Special::YouTube {
                id,
                timestamp,
                creator_name: Some(json.author_name),
                creator_url: Some(json.author_url),
            }),
            title: Some(json.title),
            description: None,
            image: None,
            video: None,
            site_name: Some(json.provider_name),
            icon_url: Some(json.thumbnail_url),
            colour: None,
        }))
    }

    pub async fn twitter(id: &str, lang: Option<String>) -> Result<Embed> {
        let mut url = format!("https://api.fxtwitter.com/2/status/{}", id);
        if let Some(lang) = lang {
            url = format!("{url}?lang={lang}");
        }
        let response = Request::new_from_str(&url)
            .await
            .map_err(|_| create_error!(ProxyError))?;

        let twitter: FxTwitterResult = serde_json::from_str(
            &response
                .response
                .text()
                .await
                .map_err(|_| create_error!(ProxyError))?,
        )
        .map_err(|_| create_error!(ProxyError))?;

        let mut maintext = twitter.status.text.clone();

        match twitter.status.quote {
            Some(FxTwitterQuote::Status(ref quote)) => {
                maintext = maintext
                    + &format!(
                        "\n\nQuoting @{}\n>>> {}",
                        &quote.author.screen_name, &quote.text
                    )
            }
            Some(FxTwitterQuote::Tombstone(ref tombstone)) => {
                maintext = maintext + &format!("\n\n>>> {}", &tombstone.message)
            }
            None => {}
        }

        let quote = twitter.status.quoted_status();

        let mut image: Option<Image> = None;
        let mut video: Option<Video> = None;

        let target_element = if twitter.status.media.all.is_some() {
            Some(&twitter.status.media)
        } else {
            quote.map(|quote| &quote.media)
        };

        if let Some(media) = target_element
            && let Some(all) = &media.all
            && let Some(first) = all.first()
        {
            match first {
                FxTwitterMediaElement::Video(video_item) => {
                    video = Some(Video {
                        url: video_item.url.clone(),
                        width: video_item.width,
                        height: video_item.height,
                    })
                }
                FxTwitterMediaElement::Photo(photo_item) => {
                    image = Some(Image {
                        url: photo_item.url.clone(),
                        width: photo_item.width,
                        height: photo_item.height,
                        size: revolt_models::v0::ImageSize::Large,
                    })
                }
            }
        }

        let url = format!(
            "https://x.com/{}/status/{id}",
            twitter.author.screen_name.clone()
        );

        let mut metadata = WebsiteMetadata {
            url: Some(url.clone()),
            original_url: Some(url),
            special: Some(Special::XApp {
                id: twitter.status.id.clone(),
                text: twitter.status.text.clone(),
                author_name: twitter.author.name.clone(),
                author_url: twitter.author.url,
                author_handle: twitter.author.screen_name.clone(),
                author_avatar_url: twitter.author.avatar_url.clone(),
                created_timestamp: twitter.status.created_timestamp,
                quote_id: quote.map(|q| q.id.clone()),
                quote_text: quote.map(|q| q.text.clone()),
                quote_author_name: quote.map(|q| q.author.name.clone()),
                quote_author_handle: quote.map(|q| q.author.screen_name.clone()),
                quote_author_url: quote.map(|q| q.author.url.clone()),
                replies: twitter.status.replies.unwrap_or(0),
                reposts: twitter.status.reposts.unwrap_or(0),
                likes: twitter.status.likes.unwrap_or(0),
                views: twitter.status.views.unwrap_or(0),
            }),
            title: Some(format!(
                "{} (@{})",
                &twitter.author.name, &twitter.author.screen_name
            )),
            description: Some(maintext),
            image,
            video,
            site_name: Some("X".to_string()),
            icon_url: twitter.author.avatar_url,
            colour: Some("#1DA1F2".to_string()),
        };

        metadata.truncate();

        Ok(Embed::Website(metadata))
    }
}
