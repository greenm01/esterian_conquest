use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests_per_window: u32,
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_window: 5,
            window_seconds: 3600,
        }
    }
}

#[derive(Debug)]
struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

pub struct RateLimiter {
    config: RateLimitConfig,
    entries: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check(&self, key: &str) -> RateLimitResult {
        let mut entries = self.entries.write().await;
        let now = Instant::now();

        if let Some(entry) = entries.get_mut(key) {
            let elapsed = now.duration_since(entry.window_start);

            if elapsed > Duration::from_secs(self.config.window_seconds) {
                entry.count = 1;
                entry.window_start = now;
                return RateLimitResult::Allowed;
            }

            if entry.count >= self.config.max_requests_per_window {
                return RateLimitResult::Rejected {
                    retry_after: self.config.window_seconds - elapsed.as_secs(),
                };
            }

            entry.count += 1;
            return RateLimitResult::Allowed;
        }

        entries.insert(
            key.to_string(),
            RateLimitEntry {
                count: 1,
                window_start: now,
            },
        );

        RateLimitResult::Allowed
    }

    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let now = Instant::now();

        entries.retain(|_, entry| {
            now.duration_since(entry.window_start)
                < Duration::from_secs(self.config.window_seconds * 2)
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitResult {
    Allowed,
    Rejected { retry_after: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_allows_within_window() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests_per_window: 3,
            window_seconds: 60,
        });

        assert_eq!(limiter.check("player1").await, RateLimitResult::Allowed);
        assert_eq!(limiter.check("player1").await, RateLimitResult::Allowed);
        assert_eq!(limiter.check("player1").await, RateLimitResult::Allowed);
    }

    #[tokio::test]
    async fn test_rate_limit_blocks_after_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests_per_window: 2,
            window_seconds: 60,
        });

        limiter.check("player1").await;
        limiter.check("player1").await;

        let result = limiter.check("player1").await;
        assert!(matches!(result, RateLimitResult::Rejected { .. }));
    }

    #[tokio::test]
    async fn test_rate_limit_separate_keys() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests_per_window: 1,
            window_seconds: 60,
        });

        assert_eq!(limiter.check("player1").await, RateLimitResult::Allowed);
        assert_eq!(limiter.check("player2").await, RateLimitResult::Allowed);
    }
}
