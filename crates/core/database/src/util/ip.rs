#[cfg(feature = "rocket-impl")]
pub mod rocket {
    use revolt_config::config;
    use rocket::Request;

    pub fn to_ip(request: &'_ Request<'_>) -> String {
        request
            .client_ip()
            .map(|x| x.to_string())
            .unwrap_or_default()
    }

    /// Find the actual IP of the client
    pub async fn to_real_ip(request: &'_ Request<'_>) -> String {
        if config().await.api.security.trust_cloudflare {
            request
                .headers()
                .get_one("CF-Connecting-IP")
                .map(|x| x.to_string())
                .unwrap_or_else(|| to_ip(request))
        } else {
            to_ip(request)
        }
    }
}

#[cfg(feature = "axum-impl")]
pub mod axum {
    use axum::{
        extract::ConnectInfo,
        http::request::Parts,
    };
    use revolt_config::config;
    use std::net::SocketAddr;

    pub fn to_ip(parts: &Parts) -> String {
        parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.ip().to_string())
            .unwrap_or_default()
    }

    /// Find the actual IP of the client
    pub async fn to_real_ip(parts: &Parts) -> String {
        if config().await.api.security.trust_cloudflare {
            parts
                .headers
                .get("CF-Connecting-IP")
                .map(|x| x.to_str().unwrap().to_string())
                .unwrap_or_else(|| to_ip(parts))
        } else {
            to_ip(parts)
        }
    }
}
