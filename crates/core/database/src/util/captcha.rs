use std::collections::HashMap;

use revolt_config::config;
use revolt_result::Result;

pub async fn check_captcha(token: Option<&str>) -> Result<()> {
    let config = config().await;

    if !config.api.security.captcha.hcaptcha_key.is_empty() {
        if let Some(token) = token {
            let mut map = HashMap::new();
            map.insert("secret", config.api.security.captcha.hcaptcha_sitekey.as_str());
            map.insert("response", token);

            let client = reqwest::Client::new();
            if let Ok(response) = client
                .post("https://hcaptcha.com/siteverify")
                .form(&map)
                .send()
                .await
            {
                #[derive(Serialize, Deserialize)]
                struct CaptchaResponse {
                    success: bool,
                }

                let result: CaptchaResponse =
                    response.json().await.map_err(|_| create_error!(CaptchaFailed))?;

                if result.success {
                    Ok(())
                } else {
                    Err(create_error!(CaptchaFailed))
                }
            } else {
                Err(create_error!(CaptchaFailed))
            }
        } else {
            Err(create_error!(CaptchaFailed))
        }
    } else {
        Ok(())
    }
}