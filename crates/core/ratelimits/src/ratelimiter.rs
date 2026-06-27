use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::ops::Add;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use dashmap::DashMap;

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
    pub map: Arc<DashMap<u64, Entry>>,
}

impl<K: RequestKind> RatelimitStorage<K> {
    pub fn new<R: for<'a> RatelimitResolver<K::R<'a>> + 'static>(resolver: R) -> Self {
        Self {
            resolver: Arc::new(resolver),
            map: Arc::new(DashMap::new()),
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
    /// Find bucket by its key
    pub fn from(map: &DashMap<u64, Entry>, key: u64) -> Entry {
        map.get(&key).map(|x| *x).unwrap_or_else(|| Entry {
            used: 0,
            reset: now().add(Duration::from_secs(10)).as_millis(),
        })
    }

    /// Deduct one unit from the bucket and save
    pub fn deduct(&mut self) {
        let current_time = now().as_millis();
        if current_time > self.reset {
            self.used = 1;
            self.reset = now().add(Duration::from_secs(10)).as_millis();
        } else {
            self.used += 1;
        }
    }

    /// Save information
    pub fn save(self, map: &DashMap<u64, Entry>, key: u64) {
        map.insert(key, self);
    }

    /// Get remaining units in the bucket
    pub fn get_remaining(&self, limit: u32) -> u32 {
        if now().as_millis() > self.reset {
            limit
        } else {
            limit - self.used
        }
    }

    /// Get how long bucket has until reset
    pub fn left_until_reset(&self) -> u128 {
        let current_time = now().as_millis();
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
    pub fn from(
        map: &DashMap<u64, Entry>,
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
        let mut entry = Entry::from(map, key);

        let remaining = entry.get_remaining(limit);
        let reset = entry.left_until_reset();
        let mut ratelimiter = Ratelimiter {
            key,
            limit,
            remaining,
            reset,
        };
        if remaining == 0 {
            return Err(ratelimiter);
        }

        entry.deduct();
        entry.save(map, key);
        ratelimiter.remaining -= 1;
        ratelimiter.reset = entry.left_until_reset();

        Ok(ratelimiter)
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum RatelimitInformation {
    Success(Ratelimiter),
    Failure { retry_after: u128 },
}
