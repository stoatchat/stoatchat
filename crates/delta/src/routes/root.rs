use std::collections::HashMap;
use revolt_config::config;
use revolt_result::Result;
use rocket::serde::json::Json;
use serde::Serialize;

/// # hCaptcha Configuration
#[derive(Serialize, JsonSchema, Debug)]
pub struct CaptchaFeature {
    /// Whether captcha is enabled
    pub enabled: bool,
    /// Client key used for solving captcha
    pub key: String,
}

/// # Generic Service Configuration
#[derive(Serialize, JsonSchema, Debug)]
pub struct Feature {
    /// Whether the service is enabled
    pub enabled: bool,
    /// URL pointing to the service
    pub url: String,
}

/// # Information about a livekit node
#[derive(Serialize, JsonSchema, Debug)]
pub struct VoiceNode {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub public_url: String,
}

/// # Voice Server Configuration
#[derive(Serialize, JsonSchema, Debug)]
pub struct VoiceFeature {
    /// Whether voice is enabled
    pub enabled: bool,
    /// All livekit nodes
    pub nodes: Vec<VoiceNode>,
}

/// # Feature Configuration
#[derive(Serialize, JsonSchema, Debug)]
pub struct RevoltFeatures {
    /// hCaptcha configuration
    pub captcha: CaptchaFeature,
    /// Whether email verification is enabled
    pub email: bool,
    /// Whether this server is invite only
    pub invite_only: bool,
    /// File server service configuration
    pub autumn: Feature,
    /// Proxy service configuration
    pub january: Feature,
    /// Voice server configuration
    pub livekit: VoiceFeature,
    /// Limits
    pub limits: LimitsConfig,
}

/// # Limits For Users
#[derive(Serialize, JsonSchema, Debug)]
pub struct LimitsConfig {
    /// Global Limits
    pub global: GlobalLimits,
    /// New User Limits
    pub new_user: UserLimits,
    /// Default User Limits
    pub default: UserLimits,
}

/// # Global limits
#[derive(Serialize, JsonSchema, Debug)]
pub struct GlobalLimits {
    /// max group size
    group_size: i64,
    /// max message embeds
    message_embeds: i64,
    /// max replies
    message_replies: i64,
    /// max reactions per message
    message_reactions: i64,
    /// max server emoji
    server_emoji: i64,
    /// max server roles
    server_roles: i64,
    /// max server channels
    server_channels: i64,
    body_limit_size: i64,
}

/// # User Limits
#[derive(Serialize, JsonSchema, Debug)]
pub struct UserLimits {
    /// Max Outgoing Friend Requests
    pub outgoing_friend_requests: i64,
    /// Max Owned Bots
    pub bots: i64,
    /// Max message content length
    pub message_length: i64,
    /// max message attachments
    pub message_attachments: i64,
    /// max servers
    pub servers: i64,
    /// max audio quality
    pub voice_quality: i64,
    /// video streaming enabled
    pub video: bool,
    /// max video resolution (vertical, horizontal)
    pub video_resolution: [i64; 2],
    /// min/max aspect ratios
    pub video_aspect_ratio: [f64; 2],
    pub file_upload_size_limits: HashMap<String, usize>,
}

impl UserLimits {
    fn from_feature_limits(fl: revolt_config::FeaturesLimits) -> UserLimits {
        UserLimits {
            outgoing_friend_requests: fl.outgoing_friend_requests as i64,
            bots: fl.bots as i64,
            message_length: fl.message_length as i64,
            message_attachments: fl.message_attachments as i64,
            servers: fl.servers as i64,
            voice_quality: fl.voice_quality as i64,
            video: fl.video,
            video_resolution: [fl.video_resolution[0] as i64, fl.video_resolution[1] as i64],
            video_aspect_ratio: [fl.video_aspect_ratio[0] as f64, fl.video_aspect_ratio[1] as f64],
            file_upload_size_limits: fl.file_upload_size_limit,
        }
    }
}

/// # Build Information
#[derive(Serialize, JsonSchema, Debug)]
pub struct BuildInformation {
    /// Commit Hash
    pub commit_sha: String,
    /// Commit Timestamp
    pub commit_timestamp: String,
    /// Git Semver
    pub semver: String,
    /// Git Origin URL
    pub origin_url: String,
    /// Build Timestamp
    pub timestamp: String,
}

/// # Server Configuration
#[derive(Serialize, JsonSchema, Debug)]
pub struct RevoltConfig {
    /// Revolt API Version
    pub revolt: String,
    /// Features enabled on this Revolt node
    pub features: RevoltFeatures,
    /// WebSocket URL
    pub ws: String,
    /// URL pointing to the client serving this node
    pub app: String,
    /// Web Push VAPID public key
    pub vapid: String,
    /// Build information
    pub build: BuildInformation,
}

/// # Query Node
///
/// Fetch the server configuration for this Revolt instance.
#[openapi(tag = "Core")]
#[get("/")]
pub async fn root() -> Result<Json<RevoltConfig>> {
    let config = config().await;

    Ok(Json(RevoltConfig {
        revolt: env!("CARGO_PKG_VERSION").to_string(),
        features: RevoltFeatures {
            captcha: CaptchaFeature {
                enabled: !config.api.security.captcha.hcaptcha_key.is_empty(),
                key: config.api.security.captcha.hcaptcha_sitekey.clone(),
            },
            email: !config.api.smtp.host.is_empty(),
            invite_only: config.api.registration.invite_only,
            autumn: Feature {
                enabled: !config.hosts.autumn.is_empty(),
                url: config.hosts.autumn.clone(),
            },
            january: Feature {
                enabled: !config.hosts.january.is_empty(),
                url: config.hosts.january.clone(),
            },
            livekit: VoiceFeature {
                enabled: !config.hosts.livekit.is_empty(),
                nodes: config
                    .api
                    .livekit
                    .nodes
                    .iter()
                    .filter(|(_, node)| !node.private)
                    .map(|(name, value)| VoiceNode {
                        name: name.clone(),
                        lat: value.lat,
                        lon: value.lon,
                        public_url: config
                            .hosts
                            .livekit
                            .get(name)
                            .expect("Missing corresponding host for voice node")
                            .clone(),
                    })
                    .collect(),
            },
            limits: LimitsConfig {
                global: GlobalLimits {
                    group_size: config.features.limits.global.group_size as i64,
                    message_embeds: config.features.limits.global.message_embeds as i64,
                    message_replies: config.features.limits.global.message_replies as i64,
                    message_reactions: config.features.limits.global.message_reactions as i64,
                    server_emoji: config.features.limits.global.server_emoji as i64,
                    server_roles: config.features.limits.global.server_roles as i64,
                    server_channels: config.features.limits.global.server_channels as i64,
                    body_limit_size: config.features.limits.global.body_limit_size as i64,
                },
                new_user: UserLimits::from_feature_limits(config.features.limits.new_user),
                default: UserLimits::from_feature_limits(config.features.limits.default),
            }
        },
        ws: config.hosts.events,
        app: config.hosts.app,
        vapid: config.pushd.vapid.public_key,
        build: BuildInformation {
            commit_sha: option_env!("VERGEN_GIT_SHA")
                .unwrap_or_else(|| "<failed to generate>")
                .to_string(),
            commit_timestamp: option_env!("VERGEN_GIT_COMMIT_TIMESTAMP")
                .unwrap_or_else(|| "<failed to generate>")
                .to_string(),
            semver: option_env!("VERGEN_GIT_SEMVER")
                .unwrap_or_else(|| "<failed to generate>")
                .to_string(),
            origin_url: option_env!("GIT_ORIGIN_URL")
                .unwrap_or_else(|| "<failed to generate>")
                .to_string(),
            timestamp: option_env!("VERGEN_BUILD_TIMESTAMP")
                .unwrap_or_else(|| "<failed to generate>")
                .to_string(),
        },
    }))
}

#[cfg(test)]
mod test {
    use crate::rocket;
    use rocket::http::Status;

    #[rocket::async_test]
    async fn hello_world() {
        let harness = crate::util::test::TestHarness::new().await;
        let response = harness.client.get("/").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn hello_world_concurrent() {
        let harness = crate::util::test::TestHarness::new().await;
        let response = harness.client.get("/").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
    }
}
