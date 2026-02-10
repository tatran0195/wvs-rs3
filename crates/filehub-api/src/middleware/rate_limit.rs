//! Token bucket rate limiter middleware.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use tokio::sync::Mutex;

/// Simple in-memory token bucket rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// IP → bucket state.
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    /// Maximum tokens per bucket.
    max_tokens: u32,
    /// Token refill rate per second.
    refill_rate: f64,
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Creates a new rate limiter.
    pub fn new(max_tokens: u32, refill_rate: f64) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            max_tokens,
            refill_rate,
        }
    }

    /// Attempts to consume a token for the given key.
    pub async fn check(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().await;
        let now = Instant::now();

        let bucket = buckets.entry(key.to_string()).or_insert(TokenBucket {
            tokens: self.max_tokens as f64,
            last_refill: now,
        });

        // Refill tokens
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.refill_rate).min(self.max_tokens as f64);
        bucket.last_refill = now;

        // Try to consume
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}
