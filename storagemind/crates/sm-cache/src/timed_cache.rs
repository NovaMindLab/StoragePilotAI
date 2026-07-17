use std::hash::Hash;
use std::time::{Duration, Instant};
use dashmap::DashMap;

/// A single entry stored in the cache.
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    ttl: Duration,
}

impl<V> CacheEntry<V> {
    #[inline]
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

/// A thread-safe, in-memory cache with per-entry TTL.
///
/// Backed by [`DashMap`] so reads and writes can happen concurrently without
/// a global lock.  Expired entries are evicted lazily on `get` and
/// opportunistically on `insert` when the map is full.
pub struct TimedCache<K: Hash + Eq, V> {
    map: DashMap<K, CacheEntry<V>>,
    default_ttl: Duration,
    max_entries: usize,
}

impl<K, V> TimedCache<K, V>
where
    K: Hash + Eq,
    V: Clone,
{
    /// Create a new cache with the given TTL and maximum entry count.
    pub fn new(default_ttl: Duration, max_entries: usize) -> Self {
        Self {
            map: DashMap::new(),
            default_ttl,
            max_entries,
        }
    }

    /// Return a clone of the cached value, or `None` if missing / expired.
    pub fn get(&self, key: &K) -> Option<V> {
        // Check presence first (shared read lock).
        let expired = self
            .map
            .get(key)
            .map(|e| e.is_expired())
            .unwrap_or(true);

        if expired {
            self.map.remove(key);
            return None;
        }

        self.map.get(key).map(|e| e.value.clone())
    }

    /// Insert `value` under `key` using the cache's default TTL.
    /// If the cache is at capacity, expired entries are swept first.
    pub fn insert(&self, key: K, value: V) {
        if self.map.len() >= self.max_entries {
            self.map.retain(|_, v| !v.is_expired());
        }
        self.map.insert(
            key,
            CacheEntry {
                value,
                inserted_at: Instant::now(),
                ttl: self.default_ttl,
            },
        );
    }

    /// Remove a single entry by key.
    pub fn invalidate(&self, key: &K) {
        self.map.remove(key);
    }

    /// Remove all entries.
    pub fn clear(&self) {
        self.map.clear();
    }

    /// Current number of entries (includes not-yet-evicted expired ones).
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let cache: TimedCache<String, i32> =
            TimedCache::new(Duration::from_secs(60), 100);
        cache.insert("key".to_string(), 42);
        assert_eq!(cache.get(&"key".to_string()), Some(42));
    }

    #[test]
    fn expired_returns_none() {
        let cache: TimedCache<String, i32> =
            TimedCache::new(Duration::from_millis(1), 100);
        cache.insert("key".to_string(), 42);
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.get(&"key".to_string()), None);
    }

    #[test]
    fn invalidate_removes_entry() {
        let cache: TimedCache<String, i32> =
            TimedCache::new(Duration::from_secs(60), 100);
        cache.insert("key".to_string(), 99);
        cache.invalidate(&"key".to_string());
        assert_eq!(cache.get(&"key".to_string()), None);
    }

    #[test]
    fn clear_empties_cache() {
        let cache: TimedCache<i32, i32> = TimedCache::new(Duration::from_secs(60), 100);
        for i in 0..10 {
            cache.insert(i, i * 2);
        }
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn evicts_expired_when_full() {
        // max_entries = 2, TTL = 1 ms
        let cache: TimedCache<i32, i32> =
            TimedCache::new(Duration::from_millis(1), 2);
        cache.insert(1, 1);
        cache.insert(2, 2);
        std::thread::sleep(Duration::from_millis(10)); // both expire
        // inserting a third should sweep expired entries, not panic
        cache.insert(3, 3);
        assert_eq!(cache.get(&3), Some(3));
    }
}
