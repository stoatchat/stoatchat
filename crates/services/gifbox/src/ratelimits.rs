use axum::http::request::Parts;
use revolt_ratelimits::ratelimiter::RatelimitResolver;

pub struct GifboxRatelimits;

impl RatelimitResolver<Parts> for GifboxRatelimits {
    fn resolve_bucket<'a>(&self, parts: &'a Parts) -> (&'a str, Option<&'a str>) {
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
            _ => ("any", None),
        }
    }

    fn resolve_bucket_limit(&self, bucket: &str) -> u32 {
        match bucket {
            "categories" => 2,
            "trending" => 5,
            "search" => 10,
            "any" => u32::MAX,
            _ => unreachable!("Bucket defined but no limit set"),
        }
    }
}
