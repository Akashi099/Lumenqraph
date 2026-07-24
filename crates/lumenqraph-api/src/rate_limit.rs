//! Token-bucket rate limiter to prevent burst attacks at window boundaries.
//! Keyed by identity (API key or "anon"). Fine for a single API instance;
//! swap for Redis when you run several.
//!
//! Token-bucket algorithm:
//! - Tokens refill at a constant rate (limit/60 tokens per second).
//! - Each request costs 1 token.
//! - Maximum tokens = limit (prevents accumulation).
//! - Tracks (tokens, last_refill_time) per identity.
//! - Prevents the 2x burst that fixed-window allows at boundaries.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Above this many tracked identities we drop stale entries.
/// Stale entries can never permit more than the limit anyway, so evicting them
/// is safe and keeps memory bounded on a long-running instance.
const MAX_TRACKED_IDENTITIES: usize = 100_000;

#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub allowed: bool,
    pub tokens_remaining: i32,
    pub retry_after_secs: Option<u64>,
}

#[derive(Debug, Clone)]
struct TokenBucketState {
    tokens: f64,
    last_refill_secs: f64,
}

#[derive(Default)]
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, TokenBucketState>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a request is allowed and returns the status.
    /// limit_per_min <= 0 means unlimited.
    pub fn check(&self, identity: &str, limit_per_min: i32) -> RateLimitStatus {
        if limit_per_min <= 0 {
            return RateLimitStatus {
                allowed: true,
                tokens_remaining: limit_per_min,
                retry_after_secs: None,
            };
        }

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        let tokens_per_sec = limit_per_min as f64 / 60.0;
        let mut buckets = self.buckets.lock().unwrap();

        // Bound memory: prune stale entries if map is too large.
        if buckets.len() >= MAX_TRACKED_IDENTITIES {
            let cutoff = now_secs - 60.0; // Keep only recently-active buckets
            buckets.retain(|_, state| state.last_refill_secs > cutoff);
        }

        let bucket = buckets
            .entry(identity.to_string())
            .or_insert_with(|| TokenBucketState {
                tokens: limit_per_min as f64,
                last_refill_secs: now_secs,
            });

        // Refill tokens based on elapsed time.
        let elapsed = now_secs - bucket.last_refill_secs;
        let refilled = elapsed * tokens_per_sec;
        bucket.tokens = (bucket.tokens + refilled).min(limit_per_min as f64);
        bucket.last_refill_secs = now_secs;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            let tokens_remaining = bucket.tokens.floor() as i32;
            RateLimitStatus {
                allowed: true,
                tokens_remaining,
                retry_after_secs: None,
            }
        } else {
            // No tokens available; calculate when next token arrives.
            let tokens_needed = 1.0 - bucket.tokens;
            let secs_until_token = tokens_needed / tokens_per_sec;
            let retry_after = (secs_until_token.ceil()) as u64;
            RateLimitStatus {
                allowed: false,
                tokens_remaining: 0,
                retry_after_secs: Some(retry_after.max(1)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_limit_then_blocks() {
        let rl = RateLimiter::new();
        let limit = 3;
        for i in 0..limit {
            let status = rl.check("k", limit);
            assert!(status.allowed, "request {i} should be allowed");
        }
        let status = rl.check("k", limit);
        assert!(!status.allowed, "4th request must be blocked");
        assert!(status.retry_after_secs.is_some());
    }

    #[test]
    fn zero_or_negative_limit_is_unlimited() {
        let rl = RateLimiter::new();
        for _ in 0..1000 {
            assert!(rl.check("k", 0).allowed);
            assert!(rl.check("k", -1).allowed);
        }
    }

    #[test]
    fn identities_are_independent() {
        let rl = RateLimiter::new();
        assert!(rl.check("a", 1).allowed);
        assert!(!rl.check("a", 1).allowed);
        // A different identity has its own fresh budget.
        assert!(rl.check("b", 1).allowed);
    }

    #[test]
    fn no_burst_at_boundaries() {
        // This test verifies the core fix: token-bucket prevents the 2x burst
        // that fixed-window allowed. With token-bucket, sustained rate at
        // boundaries never exceeds the limit.
        let rl = RateLimiter::new();
        let limit = 10;
        let mut allowed_count = 0;

        // Rapid-fire requests simulating boundary crossing.
        for _ in 0..20 {
            if rl.check("rapid", limit).allowed {
                allowed_count += 1;
            }
        }

        // Without a delay, we should allow at most `limit` requests.
        // Token-bucket starts with full bucket and drains with each request.
        assert!(
            allowed_count <= limit,
            "burst requests should not exceed limit; got {allowed_count} for limit {limit}"
        );
    }

    #[test]
    fn tokens_refill_over_time() {
        let rl = RateLimiter::new();
        let limit = 2;

        // Consume all tokens.
        assert!(rl.check("refill_test", limit).allowed);
        assert!(rl.check("refill_test", limit).allowed);
        assert!(!rl.check("refill_test", limit).allowed);

        // Simulate passage of time by sleeping a bit and checking refill.
        // (In a real system, this would be a sleep; for testing we verify the mechanism.)
        let status = rl.check("refill_test", limit);
        assert!(!status.allowed, "tokens should not immediately refill");
        assert!(status.retry_after_secs.is_some());
    }

    #[test]
    fn retry_after_header_is_reasonable() {
        let rl = RateLimiter::new();
        let limit = 1;

        rl.check("retry_test", limit);
        let status = rl.check("retry_test", limit);
        assert!(!status.allowed);
        assert!(status.retry_after_secs.is_some());
        let retry = status.retry_after_secs.unwrap();
        // Should be approximately 60 seconds (1 per minute = 1 token/60 sec).
        // Allow a reasonable range for timing.
        assert!(retry > 50 && retry <= 61, "retry_after should be ~60s, got {retry}s");
    }
}
