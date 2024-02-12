use std::collections::{hash_map::Entry, HashMap};

use crate::util::unix_timestamp;
use std::sync::Arc;
use tokio::sync::RwLockWriteGuard;
use tokio::{sync::RwLock, sync::RwLockReadGuard};

#[derive(Clone, Debug)]
pub enum CacheDuration {
    /// 1 minute
    Minute,
    /// five ,minutes
    FiveMinutes,
    ///10 minutes
    TenMinutes,

    TwentyMinutes,
    /// 1 hour,
    OneHour,
    /// 12 hours
    HalfDay,
    /// 24 hours
    Day,
    Persistant,
    /// whatever custom duration in seconds we need it to be
    Custom(i64),
}

impl CacheDuration {
    pub const fn i64(&self) -> i64 {
        match self {
            CacheDuration::Minute => 60,          // 1 minute
            CacheDuration::FiveMinutes => 300,    // 5 minutes
            CacheDuration::TenMinutes => 600,     // 10 minutes
            CacheDuration::TwentyMinutes => 1200, // 20 minutes
            CacheDuration::OneHour => 3600,       // 1 hour
            CacheDuration::HalfDay => 43200,      // 12 hours
            CacheDuration::Day => 86400,          // 24 hours,
            CacheDuration::Persistant => i64::MAX,
            CacheDuration::Custom(seconds) => *seconds,
        }
    }
}

pub type CacheValueData<T> = Arc<RwLock<T>>;

#[derive(Debug, Clone)]
pub struct CacheValue<T>
where
    T: Clone + Send + Sync,
{
    data: CacheValueData<T>,
    duration: CacheDuration,
    duration_max: CacheDuration,
    timestamp_created: i64,
    timestamp_accessed: i64,
}

impl<T> CacheValue<T>
where
    T: Clone + Send + Sync,
{
    pub fn new(input: T) -> CacheValue<T> {
        CacheValue::with_duration(input, CacheDuration::TenMinutes, CacheDuration::OneHour)
    }

    pub fn persistant(input: T) -> CacheValue<T> {
        CacheValue::with_duration(input, CacheDuration::Persistant, CacheDuration::Persistant)
    }

    pub fn exact(input: T, duration: CacheDuration) -> CacheValue<T> {
        CacheValue::with_duration(input, duration.clone(), duration)
    }

    pub fn with_duration(input: T, duration: CacheDuration, duration_max: CacheDuration) -> CacheValue<T> {
        let timestamp = unix_timestamp();
        CacheValue {
            data: Arc::from(RwLock::from(input)),
            timestamp_created: timestamp,
            timestamp_accessed: timestamp,
            duration,
            duration_max,
        }
    }

    // set the body of this cache value object
    pub async fn set(&mut self, input: T) {
        let mut writer = self.data.write().await;
        *writer = input;

        self.timestamp_created = unix_timestamp();
    }

    /// look is simliar to access, except it does not require a mutable call, which means the last access timestamp will not be updated
    /// which means it does not need a "write" operation to get to this point. So it will be faster.
    pub fn look(&self) -> Option<CacheValueData<T>> {
        if unix_timestamp() - self.timestamp_created > self.duration_max.i64() {
            None
        } else {
            Some(self.data.clone())
        }
    }

    /// Provides access if possible to the data handle.
    /// Will return None if we are outside of the max access duration
    /// this is actually a mutable call since we need to update the timestamp_accessed field
    pub fn access(&mut self) -> Option<CacheValueData<T>> {
        self.timestamp_accessed = unix_timestamp();

        if self.timestamp_accessed - self.timestamp_created > self.duration_max.i64() {
            None
        } else {
            Some(self.data.clone())
        }
    }
}

pub type CacheValueMap<T> = HashMap<String, CacheValue<T>>;

/// construct an in memory cache of whatever we want. Just a simple key value store
/// can reload from disk on first setup as well
/// key = string, value = string
#[derive(Debug, Clone, Default)]
pub struct MemoryCache<T>
where
    T: Clone + Send + Sync,
{
    data: Arc<RwLock<CacheValueMap<T>>>,
}

impl<T> MemoryCache<T>
where
    T: Clone + Send + Sync,
{
    pub fn new() -> MemoryCache<T> {
        MemoryCache {
            data: Arc::from(RwLock::from(CacheValueMap::new())),
        }
    }

    /// how many items are there in the memory cache
    pub async fn len(&self) -> usize {
        let reader = self.data.read().await;
        reader.len()
    }

    /// iterates through the memory cache and decides based on cache duration which entries to remove
    pub async fn prune(&mut self) {
        let expired = {
            let reader = self.data.read().await;
            let now = unix_timestamp();
            reader
                .iter()
                .filter_map(|(key, value)| {
                    let target_max_duration = value.duration.i64();
                    if now - value.timestamp_accessed > target_max_duration {
                        Some(key.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
        };

        if !expired.is_empty() {
            tracing::info!(
                "Found expired data in cache. Cleaning out the following keys: {:?}",
                expired
            );

            let mut writer = self.data.write().await;

            tracing::info!(
                "Before Prune:\r\nCapacity: {} || Length {}",
                writer.capacity(),
                writer.len()
            );

            for target_key in expired.iter() {
                writer.remove(target_key.as_str());
            }

            // this is crucial to perform after.
            // hash maps even when removing do not deallocate the space consumed after entry removal
            // must manually call this
            writer.shrink_to_fit();

            tracing::info!(
                "After Prune:\r\nCapacity: {} || Length {}",
                writer.capacity(),
                writer.len()
            );
        }
    }

    /// This will create the kv pair in our cache.
    ///
    /// Note: this will **overwrite** any existing value at this key position
    /// If your intention is to modify the cached value structure present at this key location then
    /// use cache.modify(...)
    pub async fn write<S: Into<String>>(&mut self, key: S, cv: CacheValue<T>) {
        let mut writer = self.data.write().await;

        // this is a little different then what you would normally do with the entry api but this avoids a "clone" call
        // not too sure how "effective" or "performant" this really is though
        // at the surface level it "seems" this will be more memory efficient then doing the full
        // entry.and_modify(|v| *v = cv.clone()).or_insert(cv) chain
        let entry = writer.entry(key.into());
        match entry {
            Entry::Occupied(_) => {
                entry.and_modify(|v| *v = cv);
            }
            Entry::Vacant(_) => {
                entry.or_insert(cv);
            }
        }
    }

    /// access a handle to the cached value structure found at this key
    /// this is internally a mutable call and requires a write operation since we need to update the timestamp_access field internally
    pub async fn access_handle(&self, key: &str) -> Option<CacheValueData<T>> {
        let mut writer = self.data.write().await;
        match writer.get_mut(key) {
            Some(cache) => cache.access(),
            _ => None,
        }
    }

    /// just fetches the data found at the matching key value. Performs any read/write operations neccessary
    pub async fn access(&self, key: &str) -> Option<T> {
        let arc = self.access_handle(key).await;
        if let Some(cache_value) = arc {
            let reader = cache_value.read().await;
            Some(reader.clone())
        } else {
            None // nothing found!
        }
    }

    /// checks for the existance of key (this is a pure read operation)
    pub async fn exist(&self, key: &str) -> bool {
        let reader = self.data.read().await;
        reader.contains_key(key)
    }

    /// apply a custom filter map operation over the current hash map
    pub async fn filter_map<F: FnMut((&String, &CacheValue<T>)) -> Option<T>>(&self, f: F) -> Vec<T> {
        let possible = {
            let reader = self.data.read().await;
            reader.iter().filter_map(f).collect::<Vec<T>>()
        };
        possible
    }

    /// lock the entire data collection for read write behavior. Full control over the hash map
    pub async fn lock_write(&self) -> RwLockWriteGuard<CacheValueMap<T>> {
        self.data.write().await
    }

    /// lock the entire data collection for read only behavior. Full readonly control over the hash map
    ///
    /// note: this will never be able update timestamp_access fields since it is strictly not mutable
    pub async fn lock_read(&self) -> RwLockReadGuard<CacheValueMap<T>> {
        self.data.read().await
    }

    /// remove from the collection the value found at the key location
    pub async fn delete(&mut self, key: &str) {
        let mut writer = self.data.write().await;
        writer.remove(key);
    }
}
