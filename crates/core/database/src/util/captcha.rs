use reqwest::Client;
use revolt_config::config;
use revolt_result::Result;
use std::sync::LazyLock;

static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

#[derive(Serialize, Deserialize)]
struct CaptchaResponse {
    success: bool,
}

pub async fn check_captcha(token: Option<&str>) -> Result<()> {
    let config = config().await;

    if !config.api.security.captcha.hcaptcha_key.is_empty() {
        let Some(token) = token else {
            return Err(create_error!(CaptchaFailed));
        };

        let response = CLIENT
            .post("https://hcaptcha.com/siteverify")
            .form(&[
                ("secret", config.api.security.captcha.hcaptcha_key.as_str()),
                ("response", token),
            ])
            .send()
            .await
            .map_err(|_| create_error!(CaptchaFailed))?
            .json::<CaptchaResponse>()
            .await
            .map_err(|_| create_error!(CaptchaFailed))?;

        if response.success {
            Ok(())
        } else {
            Err(create_error!(CaptchaFailed))
        }
    } else {
        Ok(())
    }
}
