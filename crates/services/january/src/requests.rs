use encoding_rs::{Encoding, UTF_8_INIT};
use lazy_static::lazy_static;
use mime::Mime;
use regex::Regex;
use reqwest::{
    header::{self, CONTENT_TYPE},
    redirect, Client, Response,
};
use revolt_config::report_internal_error;
use revolt_files::{create_thumbnail, decode_image, image_size_vec, is_valid_image, video_size};
use revolt_models::v0::{Embed, Image, ImageSize, Video};
use revolt_result::{create_error, Error, Result};
use std::{
    io::{Cursor, Write},
    time::Duration,
};

lazy_static! {
    /// Request client
    static ref CLIENT: Client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10)) // TODO config
        .connect_timeout(Duration::from_secs(5)) // TODO config
        .redirect(redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 5 { // TODO config
                attempt.error("too many redirects")
            } else if attempt.url().host_str() == Some("jan.revolt.chat") { // TODO config
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
        .expect("reqwest Client");

    /// Spoof User Agent as Discord
    static ref RE_USER_AGENT_SPOOFING_AS_DISCORD: Regex = Regex::new("^(?:(?:https?:)?//)?(?:(?:vx|fx)?twitter|(?:fixv|fixup)?x|(?:old\\.|new\\.|www\\.)reddit).com").expect("valid regex");

    /// Regex for matching new Reddit URLs
    static ref RE_URL_NEW_REDDIT: Regex = Regex::new("^(?:(?:https?:)?//)?(?:(?:new\\.|www\\.)?reddit).com").expect("valid regex");

    /// Cache for proxy results
    static ref PROXY_CACHE: moka::future::Cache<String, Result<(String, Vec<u8>)>> = moka::future::Cache::builder()
        .weigher(|_key, value: &Result<(String, Vec<u8>)>| -> u32 {
            std::mem::size_of::<Result<(String, Vec<u8>)>>() as u32 + if let Ok((url, vec)) = value {
                url.len().try_into().unwrap_or(u32::MAX) +
                vec.len().try_into().unwrap_or(u32::MAX)
            } else {
                std::mem::size_of::<Error>() as u32
            }
        })
        // TODO config
        .max_capacity(512 * 1024 * 1024) // Cache up to 512MiB in memory
        .time_to_live(Duration::from_secs(60)) // For up to 1 minute
        .build();

    /// Cache for embed results
    static ref EMBED_CACHE: moka::future::Cache<String, Embed> = moka::future::Cache::builder()
        // TODO config
        .max_capacity(10_000) // Cache up to 10k embeds
        .time_to_live(Duration::from_secs(60)) // For up to 1 minute
        .build();
}

/// Information about a successful request
pub struct Request {
    response: Response,
    mime: Mime,
}

impl Request {
    /// Proxy a given URL
    pub async fn proxy_file(url: &str) -> Result<(String, Vec<u8>)> {
        if let Some(hit) = PROXY_CACHE.get(url).await {
            hit
        } else {
            let Request { response, mime } = Request::new(url).await?;

            if matches!(mime.type_(), mime::IMAGE | mime::VIDEO) {
                let bytes = report_internal_error!(response.bytes().await);

                let result = match bytes {
                    Ok(bytes) => {
                        if matches!(mime.type_(), mime::IMAGE) {
                            let reader = &mut Cursor::new(&bytes);

                            if matches!(mime.subtype(), mime::GIF) {
                                if is_valid_image(reader, "image/gif") {
                                    Ok(("image/gif".to_owned(), bytes.to_vec()))
                                } else {
                                    Err(create_error!(FileTypeNotAllowed))
                                }
                            } else {
                                Ok((
                                    "image/webp".to_owned(),
                                    create_thumbnail(
                                        decode_image(reader, mime.as_ref())?,
                                        "attachments",
                                    )
                                    .await,
                                ))
                            }
                        } else {
                            let mut file = report_internal_error!(tempfile::NamedTempFile::new())?;
                            report_internal_error!(file.write_all(&bytes))?;

                            if video_size(&file).is_some() {
                                Ok((mime.to_string(), bytes.to_vec()))
                            } else {
                                Err(create_error!(FileTypeNotAllowed))
                            }
                        }
                    }
                    Err(err) => Err(err),
                };

                PROXY_CACHE.insert(url.to_owned(), result.clone()).await;
                result
            } else {
                Err(create_error!(FileTypeNotAllowed))
            }
        }
    }

    /// Fetch metadata for an image
    pub async fn fetch_image_metadata(
        url: &str,
        request: Option<Request>,
    ) -> Result<Option<Image>> {
        if let Some(hit) = EMBED_CACHE.get(url).await {
            match hit {
                Embed::Image(img) => Ok(Some(img)),
                _ => Ok(None),
            }
        } else {
            let request = if let Some(request) = request {
                request
            } else {
                let request = Request::new(url).await?;
                if matches!(request.mime.type_(), mime::IMAGE) {
                    request
                } else {
                    return Err(create_error!(FileTypeNotAllowed));
                }
            };

            if let Some((width, height)) = image_size_vec(
                &report_internal_error!(request.response.bytes().await)?,
                request.mime.as_ref(),
            ) {
                Ok(Some(Image {
                    url: url.to_owned(),
                    width,
                    height,
                    size: ImageSize::Large,
                }))
            } else {
                Ok(None)
            }
        }
    }

    /// Fetch metadata for an video
    pub async fn fetch_video_metadata(
        url: &str,
        request: Option<Request>,
    ) -> Result<Option<Video>> {
        if let Some(hit) = EMBED_CACHE.get(url).await {
            match hit {
                Embed::Video(vid) => Ok(Some(vid)),
                _ => Ok(None),
            }
        } else {
            let response = if let Some(Request { response, .. }) = request {
                response
            } else {
                let Request { response, mime } = Request::new(url).await?;
                if matches!(mime.type_(), mime::VIDEO) {
                    response
                } else {
                    return Err(create_error!(FileTypeNotAllowed));
                }
            };

            let mut file = report_internal_error!(tempfile::NamedTempFile::new())?;
            report_internal_error!(
                file.write_all(&report_internal_error!(response.bytes().await)?)
            )?;

            if let Some((width, height)) = video_size(&file) {
                Ok(Some(Video {
                    url: url.to_owned(),
                    width: width as usize,
                    height: height as usize,
                }))
            } else {
                Ok(None)
            }
        }
    }

    /// Generate embed for a given URL
    pub async fn generate_embed(mut url: String) -> Result<Embed> {
        // Re-map certain links for better metadata generation
        if RE_URL_NEW_REDDIT.is_match(&url) {
            url = RE_URL_NEW_REDDIT
                // Reddit has a bunch of clickbait-y marketing on the new URLs, so we use the old site instead
                .replace(&url, "https://old.reddit.com")
                .to_string();
        }

        // Generate the actual embed
        if let Some(hit) = EMBED_CACHE.get(&url).await {
            Ok(hit)
        } else {
            let request = Request::new(&url).await?;
            let embed = match (request.mime.type_(), request.mime.subtype()) {
                (_, mime::HTML) => {
                    let content_type = request
                        .response
                        .headers()
                        .get(header::CONTENT_TYPE)
                        .and_then(|value| value.to_str().ok())
                        .and_then(|value| value.parse::<Mime>().ok());

                    let encoding_name = content_type
                        .as_ref()
                        .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
                        .unwrap_or("utf-8");

                    let encoding =
                        Encoding::for_label(encoding_name.as_bytes()).unwrap_or(&UTF_8_INIT);

                    let bytes = report_internal_error!(request.response.bytes().await)?;
                    let (text, _, _) = encoding.decode(&bytes);

                    crate::website_embed::create_website_embed(&url, &text)
                        .await
                        .map(Embed::Website)
                        .unwrap_or_default()
                }
                (mime::IMAGE, _) if url.contains("static.klipy.com/") => {
                    match Request::fetch_image_metadata(&url, Some(request)).await {
                        Ok(Some(img)) => Embed::Website(revolt_models::v0::WebsiteMetadata {
                            url: Some(url.clone()),
                            original_url: Some(url.clone()),
                            special: Some(revolt_models::v0::Special::GIF),
                            title: None,
                            description: None,
                            image: Some(img),
                            video: None,
                            site_name: None,
                            icon_url: None,
                            colour: None,
                        }),
                        _ => Embed::None,
                    }
                }
                (mime::IMAGE, _) => Request::fetch_image_metadata(&url, Some(request))
                    .await
                    .map(|res| res.map(Embed::Image).unwrap_or_default())
                    .unwrap_or_default(),
                (mime::VIDEO, _) => Request::fetch_video_metadata(&url, Some(request))
                    .await
                    .map(|res| res.map(Embed::Video).unwrap_or_default())
                    .unwrap_or_default(),
                _ => Embed::None,
            };

            EMBED_CACHE.insert(url.to_owned(), embed.clone()).await;
            Ok(embed)
        }
    }

    /// Send a new request to a service
    pub async fn new(url: &str) -> Result<Request> {
        let response = CLIENT
            .get(url)
            .header(
                "User-Agent",
                if RE_USER_AGENT_SPOOFING_AS_DISCORD.is_match(url) {
                    "Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)"
                } else {
                    "Mozilla/5.0 (compatible; January/2.0; +https://github.com/revoltchat/backend)"
                },
            )
            .header("Accept-Language", "en-US,en;q=0.5")
            .send()
            .await
            .map_err(|_| create_error!(ProxyError))?;

        if !response.status().is_success() {
            tracing::error!("{:?}", response);
            return Err(create_error!(ProxyError));
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .ok_or(create_error!(ProxyError))?
            .to_str()
            .map_err(|_| create_error!(ProxyError))?;

        let mime: mime::Mime = content_type
            .parse()
            .map_err(|_| create_error!(ProxyError))?;

        Ok(Request { response, mime })
    }

    /// Check if something exists
    pub async fn exists(url: &str) -> bool {
        if let Ok(response) = CLIENT.head(url).send().await {
            response.status().is_success()
        } else {
            false
        }
    }
}
