use revolt_config::{ApiRatelimitOverride, Ratelimit, config};
use revolt_database::User;
use revolt_ratelimits::ratelimiter::{RatelimitResolver, async_trait};
use rocket::{Request, http::Method};

pub struct DeltaRatelimits;

fn get_bucket_overrides(
    bucket: &str,
    default: Ratelimit,
    overrides: Vec<&ApiRatelimitOverride>,
) -> Ratelimit {
    // When several overrides match, use the most permissive one
    overrides
        .into_iter()
        .filter_map(|o| o.overrides.get(bucket).copied())
        .max_by(|(a_limit, a_window), (b_limit, b_window)| {
            (a_limit * b_window).cmp(&(b_limit * a_window))
        })
        .unwrap_or(default)
}

#[async_trait]
impl<'a> RatelimitResolver<Request<'a>> for DeltaRatelimits {
    async fn resolve_bucket<'r>(
        &self,
        request: &'r Request<'a>,
        _user: Option<&User>,
    ) -> (&'r str, Option<&'r str>) {
        let (segment, resource, extra) = (
            request.routed_segment(0),
            request.routed_segment(1),
            request.routed_segment(2),
        );

        if let Some(segment) = segment {
            #[allow(clippy::redundant_locals)]
            let resource = resource;

            let method = request.method();
            match (segment, resource, extra, method) {
                ("users", target, _, Method::Patch) => ("users/patch", target),
                ("users", target, _, _) => {
                    if let Some("default_avatar") = extra {
                        return ("users/<id>/default_avatar", target);
                    }

                    ("users", None)
                }
                ("bots", _, _, _) => ("bots", None),
                ("channels", Some(id), extra, _) => {
                    if method == Method::Post
                        && let Some("messages") = extra
                    {
                        return ("channels/<id>/post", Some(id));
                    }

                    ("channels/<id>", Some(id))
                }
                ("servers", Some(id), Some("members"), Method::Delete)
                | ("servers", Some(id), Some("members"), Method::Patch)
                | ("servers", Some(id), Some("bans"), _) => ("servers/<id>/mod", Some(id)),
                ("servers", Some(id), _, _) => ("servers/<id>", Some(id)),
                ("auth", _, _, _) => {
                    if method == Method::Delete {
                        ("auth/delete", None)
                    } else {
                        ("auth", None)
                    }
                }
                ("swagger", _, _, _) => ("swagger", None),
                ("safety", Some("report"), _, _) => ("safety/report", None),
                _ => ("fallback", None),
            }
        } else {
            ("fallback", None)
        }
    }

    async fn resolve_bucket_limit(&self, bucket: &str, user: Option<&User>) -> (u32, u32) {
        let config = config().await;
        let default: &Ratelimit = config
            .api
            .ratelimits
            .defaults
            .get(bucket)
            .unwrap_or(&config.api.ratelimits.fallback);

        if let Some(user) = user
            && let Some(overrides) = &user.ratelimit_overrides
        {
            let applicable_overrides: Vec<&ApiRatelimitOverride> = config
                .api
                .ratelimits
                .overrides
                .iter()
                .filter(|o| overrides.contains(&o.key))
                .collect();

            get_bucket_overrides(bucket, *default, applicable_overrides)
        } else {
            *default
        }
    }

    fn apply_bucket(&self, bucket: &str, resource: Option<&str>) -> String {
        if let Some(rss) = resource {
            bucket.replace("<id>", rss)
        } else {
            bucket.to_string()
        }
    }
}
