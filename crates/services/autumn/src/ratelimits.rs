use axum::http::{Method, request::Parts};
use revolt_database::User;
use revolt_ratelimits::ratelimiter::{RatelimitResolver, async_trait};

pub struct AutumnRatelimits;

#[async_trait]
impl RatelimitResolver<Parts> for AutumnRatelimits {
    async fn resolve_bucket<'a>(
        &self,
        parts: &'a Parts,
        _user: Option<&User>,
    ) -> (&'a str, Option<&'a str>) {
        let path = parts
            .uri
            .path()
            .trim_matches('/')
            .split_terminator("/")
            .collect::<Vec<&str>>();

        match (&parts.method, path.as_slice()) {
            (&Method::POST, &[tag]) => ("upload", Some(tag)),
            _ => ("any_atmn", None),
        }
    }

    async fn resolve_bucket_limit(&self, bucket: &str, _user: Option<&User>) -> (u32, u32) {
        let limit = match bucket {
            "upload" => 10,
            _ => u32::MAX,
        };

        (limit, 10)
    }

    fn apply_bucket(&self, bucket: &str, resource: Option<&str>) -> String {
        bucket.to_string()
    }
}
