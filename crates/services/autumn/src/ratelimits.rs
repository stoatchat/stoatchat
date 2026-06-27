use axum::http::{request::Parts, Method};
use revolt_ratelimits::ratelimiter::RatelimitResolver;

pub struct AutumnRatelimits;

impl RatelimitResolver<Parts> for AutumnRatelimits {
    fn resolve_bucket<'a>(&self, parts: &'a Parts) -> (&'a str, Option<&'a str>) {
        let path = parts
            .uri
            .path()
            .trim_matches('/')
            .split_terminator("/")
            .collect::<Vec<&str>>();

        match (&parts.method, path.as_slice()) {
            (&Method::POST, &[tag]) => ("upload", Some(tag)),
            _ => ("any", None),
        }
    }

    fn resolve_bucket_limit(&self, bucket: &str) -> u32 {
        match bucket {
            "upload" => 10,
            "any" => u32::MAX,
            _ => unreachable!("Bucket defined but no limit set"),
        }
    }
}
