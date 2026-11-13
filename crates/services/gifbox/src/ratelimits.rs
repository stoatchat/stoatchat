use axum::http::request::Parts;
use revolt_database::User;
use revolt_ratelimits::ratelimiter::{RatelimitResolver, async_trait};

pub struct GifboxRatelimits;

#[async_trait]
impl RatelimitResolver<Parts> for GifboxRatelimits {
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
            .next();

        match path {
            Some("categories") => ("categories", None),
            Some("trending") => ("trending", None),
            Some("search") => ("search", None),
            _ => ("any_gfbx", None),
        }
    }

    async fn resolve_bucket_limit(&self, bucket: &str, _user: Option<&User>) -> (u32, u32) {
        let limit = match bucket {
            "categories" => 2,
            "trending" => 5,
            "search" => 10,
            _ => u32::MAX,
        };

        (limit, 10)
    }

    fn apply_bucket(&self, bucket: &str, resource: Option<&str>) -> String {
        bucket.to_string()
    }
}
