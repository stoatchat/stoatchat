use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::ops::Add;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redis_kiss::redis::ExistenceCheck;
use redis_kiss::{
    get_connection,
    redis::{Pipeline, SetExpiry, SetOptions, aio::Connection},
};
use revolt_result::ToRevoltError;
use serde::Serialize;

static IS_TEST_ENV: LazyLock<bool> = LazyLock::new(|| std::env::var("TEST_DB").is_ok());

pub trait RequestKind {
    type R<'a>;
}

pub trait RatelimitResolver<R>: Send + Sync {
    fn resolve_bucket<'a>(&self, request: &'a R) -> (&'a str, Option<&'a str>);
    fn resolve_bucket_limit(&self, bucket: &str) -> u32;
}

#[derive(Clone)]
pub struct RatelimitStorage<K: RequestKind> {
    pub resolver: Arc<dyn for<'a> RatelimitResolver<K::R<'a>>>,
}

impl<K: RequestKind> RatelimitStorage<K> {
    pub fn new<R: for<'a> RatelimitResolver<K::R<'a>> + 'static>(resolver: R) -> Self {
        Self {
            resolver: Arc::new(resolver),
        }
    }
}

/// Ratelimit Bucket
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    used: u32,
    reset: u128,
}

/// Get the current time from Unix Epoch as a Duration
fn now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards...")
}

impl Entry {
    /// Find bucket by its key, and incremement its usage count
    pub async fn from(conn: &mut Connection, now: Duration, key: u64) -> Entry {
        let expire = now.add(Duration::from_secs(10)).as_millis();

        let (used, _, reset) = Pipeline::new()
            .incr(format!("rl:{key}:used"), 1)
            .set_options(
                format!("rl:{key}:reset"),
                expire as usize,
                SetOptions::default()
                    .conditional_set(ExistenceCheck::NX)
                    .with_expiration(SetExpiry::EX(10)),
            )
            .get(format!("rl:{key}:reset"))
            .query_async::<_, (u32, (), u128)>(conn)
            .await
            .ok()
            .unwrap_or((1, (), expire));

        Entry { used, reset }
    }

    /// Check if the entry is expired, and reset it if so
    pub fn is_expired(&mut self, now: Duration) {
        let current_time = now.as_millis();

        if current_time > self.reset {
            self.used = 1;
            self.reset = now.add(Duration::from_secs(10)).as_millis();
        };
    }

    /// Save information
    pub async fn save(self, conn: &mut Connection, key: u64) {
        let _ = Pipeline::new()
            .pexpire_at(format!("rl:{key}:used"), self.reset as usize)
            .pexpire_at(format!("rl:{key}:reset"), self.reset as usize)
            .query_async::<_, ()>(conn)
            .await
            .to_internal_error();
    }

    /// Get remaining units in the bucket
    pub fn get_remaining(&self, now: Duration, limit: u32) -> u32 {
        if now.as_millis() > self.reset {
            limit
        } else {
            (limit.saturating_add(1)).saturating_sub(self.used)
        }
    }

    /// Get how long bucket has until reset
    pub fn left_until_reset(&self, now: Duration) -> u128 {
        let current_time = now.as_millis();
        self.reset.saturating_sub(current_time)
    }
}

/// Ratelimit Guard
#[derive(Serialize, Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct Ratelimiter {
    pub key: u64,
    pub limit: u32,
    pub remaining: u32,
    pub reset: u128,
}

impl Ratelimiter {
    /// Generate guard from identifier and target bucket
    pub async fn from(
        identifier: &str,
        limit: u32,
        (bucket, resource): (&str, Option<&str>),
    ) -> Result<Ratelimiter, Ratelimiter> {
        let mut key = DefaultHasher::new();
        key.write(identifier.as_bytes());
        key.write(bucket.as_bytes());

        if let Some(id) = resource {
            key.write(id.as_bytes());
        }

        let key = key.finish();

        if *IS_TEST_ENV {
            return Ok(Ratelimiter {
                key,
                limit,
                remaining: limit,
                reset: 10000,
            });
        }

        let mut conn = get_connection()
            .await
            .expect("Failed to get redis connection")
            .into_inner();

        let now = now();
        let mut entry = Entry::from(&mut conn, now, key).await;

        let remaining = entry.get_remaining(now, limit);
        let reset = entry.left_until_reset(now);
        let mut ratelimiter = Ratelimiter {
            key,
            limit,
            remaining,
            reset,
        };
        if remaining == 0 {
            return Err(ratelimiter);
        }

        entry.is_expired(now);
        entry.save(&mut conn, key).await;
        ratelimiter.remaining = ratelimiter.remaining.saturating_sub(1);

        Ok(ratelimiter)
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum RatelimitInformation {
    Success(Ratelimiter),
    Failure { retry_after: u128 },
}
