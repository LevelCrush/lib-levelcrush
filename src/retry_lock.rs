use std::time::Duration;

use crate::{
    alias::UnixTimestamp,
    cache::{CacheValue, MemoryCache},
    util::unix_timestamp,
};

#[derive(Debug, Clone)]
pub struct RetryLock {
    cache: MemoryCache<UnixTimestamp>,
    max_retries: usize,
    duration_try: Duration,
}

impl RetryLock {
    pub fn new(retries: usize, wait_duration: Duration) -> RetryLock {
        RetryLock {
            cache: MemoryCache::new(),
            max_retries: retries,
            duration_try: wait_duration,
        }
    }

    /// waits until the lock is released (or until max retries are hit)
    pub async fn wait_until_release(&mut self, key: &str) -> usize {
        let mut retries = 0;
        'retry_lock: while retries < self.max_retries {
            if self.cache.exist(key).await {
                tokio::time::sleep(self.duration_try).await;
                retries += 1;
            } else {
                break 'retry_lock;
            }
        }
        retries
    }

    /// checks to see if there is a lock found that matches the key
    pub async fn check(&self, key: &str) -> bool {
        self.cache.exist(key).await
    }

    /// locks the key from being accessed (until max retries are reached)
    /// returns the amount of retries before it was allowed to lock
    pub async fn lock(&mut self, key: &str) -> usize {
        //before we can lock, we must make sure that we are able to
        let retries = self.wait_until_release(key).await;
        if retries > 0 {
            tracing::info!("Had to wait for release for {} attempts, at {}", retries, key);
        }
        self.cache.write(key, CacheValue::persistant(unix_timestamp())).await;

        retries
    }

    /// unlocks the key and lets the application continue processing the next block of logic
    pub async fn unlock(&mut self, key: &str) {
        self.cache.delete(key).await;
    }
}

impl Default for RetryLock {
    fn default() -> Self {
        Self::new(10, Duration::from_secs(1))
    }
}
