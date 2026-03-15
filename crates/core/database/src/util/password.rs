use std::{collections::HashSet, sync::LazyLock};
use sha1::Digest;

use revolt_config::config;
use revolt_result::{ToRevoltError, Result};


static ARGON_CONFIG: LazyLock<argon2::Config<'static>> = LazyLock::new(argon2::Config::default);
static TOP_100K_COMPROMISED: LazyLock<HashSet<String>> = LazyLock::new(|| include_str!("../../assets/pwned100k.txt")
        .split('\n')
        .map(|x| x.into())
        .collect());

/// Hash a password using argon2
pub fn hash_password(plaintext_password: String) -> Result<String> {
    argon2::hash_encoded(
        plaintext_password.as_bytes(),
        nanoid::nanoid!(24).as_bytes(),
        &ARGON_CONFIG,
    )
    .to_internal_error()
}

pub async fn assert_safe(password: &str) -> Result<()> {
    // Make sure the password is long enough.
    if password.len() < 8 {
        return Err(create_error!(ShortPassword));
    }

    let config = config().await;

    if !config.api.security.easypwned.is_empty() {
        let mut hasher = sha1::Sha1::new();
        hasher.update(password);
        let pwd_hash = hasher.finalize();

        #[derive(Deserialize)]
        struct EasyPwnedResult {
            secure: bool,
        }

        let result = match reqwest::get(format!("{}/hash/{pwd_hash:#02x}", &config.api.security.easypwned)).await {
            Ok(response) => match response.json::<EasyPwnedResult>().await {
                Ok(result) => Ok(result.secure),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        };

        if let Err(e) = &result {
            revolt_config::capture_error(e);
        } else if result.is_ok_and(|b| b) {
            return Ok(())
        }

        Err(create_error!(CompromisedPassword))
    } else if TOP_100K_COMPROMISED.contains(password) {
        Err(create_error!(CompromisedPassword))
    } else {
        Ok(())
    }
}