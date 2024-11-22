use std::time::Instant;

use tokio::sync::RwLock;

use super::RateLimitState;

pub trait RateLimitStateTrait {
    fn init(max_query_per_mn: usize) -> Self;
    async fn add_entry(&self);
    async fn can_fetch(&self) -> bool;
}

impl RateLimitStateTrait for RateLimitState {
    fn init(max_query_per_mn: usize) -> Self {
        RateLimitState {
            max_query_per_mn,
            data: RwLock::new(Vec::new()),
        }
    }

    async fn add_entry(&self) {
        self.with_limiter(|data| {
            let now = Instant::now();

            data.retain(|&time| now.duration_since(time).as_secs() < 60);

            data.push(now);
        })
        .await;
    }

    async fn can_fetch(&self) -> bool {
        self.with_limiter(|data| {
            let now = Instant::now();
            let mut count = 0;
            data.retain(|&time| {
                if now.duration_since(time).as_secs() < 60 {
                    count += 1;
                    true
                } else {
                    false
                }
            });
            count < self.max_query_per_mn
        })
        .await
    }
}
