use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use nanoid::nanoid;
use redis_kiss::{AsyncCommands, get_connection};
use revolt_config::config;
use revolt_result::{Result, ToRevoltError};
use tokio::sync::OnceCell;
use webauthn_rs::prelude::{
    DiscoverableAuthentication, PasskeyAuthentication, PasskeyRegistration, Url, Webauthn,
    WebauthnBuilder,
};

/// How long a passkey ceremony may take before it expires
const CHALLENGE_TTL: usize = 300;

static WEBAUTHN: OnceCell<Webauthn> = OnceCell::const_new();

pub async fn webauthn() -> Result<&'static Webauthn> {
    WEBAUTHN.get_or_try_init(build_webauthn).await
}

async fn build_webauthn() -> Result<Webauthn> {
    let config = config().await;

    let origin = Url::parse(&config.hosts.app).to_internal_error()?;

    let rp_id = if config.api.security.webauthn.rp_id.is_empty() {
        origin
            .host_str()
            .ok_or_else(|| create_error!(InternalError))?
            .to_string()
    } else {
        config.api.security.webauthn.rp_id.clone()
    };

    let mut builder = WebauthnBuilder::new(&rp_id, &origin)
        .to_internal_error()?
        .rp_name(&config.api.security.webauthn.rp_name);

    for extra_origin in &config.api.security.webauthn.extra_origins {
        builder = builder.append_allowed_origin(&Url::parse(extra_origin).to_internal_error()?);
    }

    builder.build().to_internal_error()
}

/// helper to make sure everything is encoded the same.
pub(crate) fn encode_credential_id(credential_id: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(credential_id)
}

/// Temporary state of an in-progress passkey ceremony
#[derive(Serialize, Deserialize)]
pub struct PasskeyChallenge {
    /// Account this ceremony belongs to, `None` for discoverable (passwordless) login
    pub account_id: Option<String>,
    pub state: PasskeyChallengeState,
}

#[derive(Serialize, Deserialize)]
pub enum PasskeyChallengeState {
    Registration(PasskeyRegistration),
    Authentication(PasskeyAuthentication),
    Discoverable(DiscoverableAuthentication),
}

impl PasskeyChallenge {
    /// Store this challenge in redis and return the token the client must send back
    pub async fn save(&self) -> Result<String> {
        let token = nanoid!(64);
        let value = serde_json::to_string(self).to_internal_error()?;

        get_connection()
            .await
            .map_err(|_| create_error!(InternalError))?
            .set_ex::<_, _, ()>(format!("passkey_challenge:{token}"), value, CHALLENGE_TTL)
            .await
            .to_internal_error()?;

        Ok(token)
    }

    /// Get and delete the token from redis
    pub async fn take(token: &str) -> Result<PasskeyChallenge> {
        let value: Option<String> = get_connection()
            .await
            .map_err(|_| create_error!(InternalError))?
            .get_del(format!("passkey_challenge:{token}"))
            .await
            .to_internal_error()?;

        value
            .and_then(|value| serde_json::from_str(&value).ok())
            .ok_or_else(|| create_error!(InvalidToken))
    }
}
