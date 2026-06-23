use encoding_rs::{Encoding, UTF_8_INIT};
use lazy_static::lazy_static;
use mime::Mime;
use pdk_ip_filter_lib::IpFilter;
use regex::Regex;
use reqwest::{
    dns::{Addrs, Name, Resolve},
    header::{self, CONTENT_TYPE},
    redirect, Client, Response,
};
use revolt_config::{config, report_internal_error};
use revolt_files::{create_thumbnail, decode_image, image_size_vec, is_valid_image, video_size};
use revolt_models::v0::{Embed, Image, ImageSize, Video};
use revolt_result::{create_error, Error, Result, ToRevoltError};
use std::net::{IpAddr, SocketAddr};
use std::{
    io::{Cursor, Write},
    str::FromStr,
    time::Duration,
};
use url::{Host, Url};

lazy_static! {
    /// Request client
    static ref CLIENT: Client = reqwest::Client::builder()
        .dns_resolver(CachedDnsResolver {})
        .timeout(Duration::from_secs(10)) // TODO config
        .connect_timeout(Duration::from_secs(5)) // TODO config
        .redirect(redirect::Policy::none())
        .build()
        .expect("reqwest Client");

    /// Spoof User Agent as Discord
    static ref RE_USER_AGENT_SPOOFING_AS_DISCORD: Regex = Regex::new("^(?:(?:vx|fx)?twitter|(?:fixv|fixup)?x|(?:old\\.|new\\.|www\\.)reddit).com").expect("valid regex");

    /// Regex for matching new Reddit URLs
    static ref RE_URL_NEW_REDDIT: Regex = Regex::new("^(?:(?:new\\.|www\\.)?reddit).com").expect("valid regex");

    /// Regex for matching YouTube Shorts URLs
    static ref RE_URL_YOUTUBE_SHORTS: Regex = Regex::new("^(?:(?:https?:)?//)?(?:(?:www\\.)?youtube\\.com)/shorts/([a-zA-Z0-9_-]+)").expect("valid regex");

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

    static ref DNS_CACHE: moka::future::Cache<String, Vec<SocketAddr>> = moka::future::Cache::builder()
        .max_capacity(10_000)
        .time_to_idle(Duration::from_secs(30))
        .build();

    static ref IP_BLOCKLIST: IpFilter = IpFilter::block(&[
        "0.0.0.0/8",
        "10.0.0.0/8",
        "192.168.0.0/16",
        "127.0.0.0/8",
        "172.16.0.0/12",
        "169.254.0.0/16",
        "::1",
        "fc00::/7",
        ]
    ).unwrap();
}

#[derive(Clone)]
pub struct IPRequest {
    url: Url,
    ip: IpAddr,
    pub blocked: bool,
}

impl From<IPRequest> for Url {
    fn from(value: IPRequest) -> Self {
        let mut url = value.url.clone();
        url.set_host(Some(&value.ip.to_string()))
            .map(|_| url)
            .unwrap_or(value.url)
    }
}

struct CachedDnsResolver {}

impl reqwest::dns::Resolve for CachedDnsResolver {
    fn resolve(&self, name: Name) -> reqwest::dns::Resolving {
        Box::pin(async move {
            {
                if let Some(addrs) = DNS_CACHE.get(&name.as_str().to_string()).await {
                    let resp: Addrs = Box::new(addrs.clone().into_iter());
                    return Ok(resp);
                }
            }

            let mut lookup = name.as_str().to_string();
            if !lookup.contains(":") {
                lookup += ":0";
            }

            let fallback: Vec<SocketAddr> = tokio::net::lookup_host(&lookup)
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?
                .collect();

            {
                DNS_CACHE
                    .insert(name.as_str().to_string().clone(), fallback.clone())
                    .await;
                let addrs: Addrs = Box::new(fallback.clone().into_iter());
                Ok(addrs)
            }
        })
    }
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
            let Request { response, mime } = Request::new_from_str(url).await?;

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
                let request = Request::new_from_str(url).await?;
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
                let Request { response, mime } = Request::new_from_str(url).await?;
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

        // Re-map Youtube Shorts to regular Youtube links
        if let Some(captures) = RE_URL_YOUTUBE_SHORTS.captures(&url) {
            if let Some(video_id) = captures.get(1) {
                url = format!("https://youtube.com/watch?v={}", video_id.as_str());
            }
        }

        // Generate the actual embed
        if let Some(hit) = EMBED_CACHE.get(&url).await {
            Ok(hit)
        } else {
            let request = Request::new_from_str(&url).await?;
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
    pub async fn new(url: Url) -> Result<Request> {
        let mut url = url;
        let url_host_str = url.host_str().ok_or(create_error!(ProxyError))?.to_string();

        let mut blocker = Request::url_is_blacklisted(&url).await?;

        if blocker.blocked {
            return Err(create_error!(InvalidOperation));
        }

        let mut redirect_count = 0;

        loop {
            let response = CLIENT
            .get(url)
            .header(
                "User-Agent",
                if RE_USER_AGENT_SPOOFING_AS_DISCORD.is_match(&url_host_str) {
                    "Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)"
                } else {
                    "Mozilla/5.0 (compatible; January/2.0; +https://github.com/stoatchat/stoatchat)"
                },
            )
            .header("Accept-Language", "en-US,en;q=0.5")
            .send()
            .await
            .map_err(|_| create_error!(ProxyError))?;

            if response.status().is_redirection() {
                redirect_count += 1;

                if redirect_count > 5 {
                    return Err(create_error!(ProxyError));
                }
                if let Some(location) = response.headers().get("location") {
                    let location = location.to_str().map_err(|_| create_error!(ProxyError))?;
                    url = Url::from_str(location).to_internal_error()?;

                    blocker = Request::url_is_blacklisted(&url).await?;

                    if blocker.blocked {
                        return Err(create_error!(InvalidOperation));
                    }

                    continue;
                } else {
                    return Err(create_error!(ProxyError));
                }
            }

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

            return Ok(Request { response, mime });
        }
    }

    pub async fn new_from_str(url: &str) -> Result<Request> {
        let proper_url = Url::parse(url).map_err(|_| create_error!(ProxyError))?;
        Request::new(proper_url).await
    }

    /// Check if something exists
    pub async fn exists(url: Url) -> bool {
        if let Ok(response) = CLIENT.head(url).send().await {
            response.status().is_success()
        } else {
            false
        }
    }

    pub async fn exists_from_str(url: &str) -> Result<bool> {
        let proper_url = Url::parse(url).map_err(|_| create_error!(ProxyError))?;
        Ok(Request::exists(proper_url).await)
    }

    pub async fn url_is_blacklisted(url: &Url) -> Result<IPRequest> {
        let resolved_address: IpAddr;

        if let Some(host) = url.host() {
            match host {
                Host::Ipv4(ipv4) => {
                    resolved_address = ipv4.into();
                    if !IP_BLOCKLIST.is_allowed(&ipv4.to_string()) {
                        return Err(create_error!(InvalidOperation));
                    }
                }
                Host::Ipv6(ipv6) => {
                    resolved_address = ipv6.into();
                    if !IP_BLOCKLIST.is_allowed(&ipv6.to_string()) {
                        return Err(create_error!(InvalidOperation));
                    }
                }
                Host::Domain(domain) => {
                    let domain = domain.to_string();

                    let config = config().await;

                    // First step: TLDs and blocked domains
                    if !domain.contains(".") // lazily block TLDs
                        || config.january.blocked_domains.iter().any(|x| x == &domain)
                    {
                        return Err(create_error!(InvalidOperation));
                    }

                    // Second step: resolve the IP and check the blocklist
                    let resolver = CachedDnsResolver {};
                    if let Ok(mut resolved_ip) = resolver
                        .resolve(
                            Name::from_str(&domain)
                                .map_err(|_| create_error!(ProxyError))
                                .unwrap(),
                        )
                        .await
                    {
                        if let Some(resolved_ip) = resolved_ip.next() {
                            resolved_address = resolved_ip.ip();
                            let resolved_string = resolved_address.to_string();
                            if !IP_BLOCKLIST.is_allowed(&resolved_string)
                                || resolved_string.contains("::ffff:")
                            {
                                return Err(create_error!(InvalidOperation));
                            }
                        } else {
                            return Err(create_error!(InvalidOperation));
                        }
                    } else {
                        return Err(create_error!(ProxyError));
                    }
                }
            }
        } else {
            return Err(create_error!(ProxyError));
        };

        Ok(IPRequest {
            url: url.clone(),
            ip: resolved_address,
            blocked: false,
        })
    }
}
