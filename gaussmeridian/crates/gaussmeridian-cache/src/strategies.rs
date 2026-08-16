//! Cache eviction strategies

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::Hash;
use std::time::{Duration, Instant};

/// Cache eviction strategy trait
pub trait EvictionStrategy<K: Eq> {
    /// Add a key to the eviction strategy
    fn add(&mut self, key: K);

    /// Update a key's access in the eviction strategy
    fn update(&mut self, key: &K) -> bool;

    /// Get the next key to evict
    fn evict(&mut self) -> Option<K>;

    /// Remove a key from the eviction strategy
    fn remove(&mut self, key: &K) -> bool;

    /// Get the number of keys in the strategy
    fn len(&self) -> usize;

    /// Check if the strategy is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Least Recently Used (LRU) eviction strategy
pub struct LRUStrategy<K> {
    queue: VecDeque<K>,
    access_map: HashMap<K, usize>,
}

impl<K> LRUStrategy<K>
where
    K: Clone + Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            access_map: HashMap::new(),
        }
    }
}

impl<K> EvictionStrategy<K> for LRUStrategy<K>
where
    K: Clone + Eq + Hash,
{
    fn add(&mut self, key: K) {
        self.queue.push_back(key.clone());
        self.access_map.insert(key, self.queue.len() - 1);
    }

    fn update(&mut self, key: &K) -> bool {
        if let Some(&pos) = self.access_map.get(key) {
            // Remove from current position
            self.queue.remove(pos);
            // Add to end (most recently used)
            self.queue.push_back(key.clone());
            // Update position
            self.access_map.insert(key.clone(), self.queue.len() - 1);
            true
        } else {
            false
        }
    }

    fn evict(&mut self) -> Option<K> {
        self.queue.pop_front().map(|key| {
            self.access_map.remove(&key);
            key
        })
    }

    fn remove(&mut self, key: &K) -> bool {
        if let Some(&pos) = self.access_map.get(key) {
            self.queue.remove(pos);
            self.access_map.remove(key);
            true
        } else {
            false
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

/// Least Frequently Used (LFU) eviction strategy
pub struct LFUStrategy<K> {
    frequency_map: HashMap<K, u64>,
    frequency_tree: BTreeMap<u64, Vec<K>>,
}

impl<K> LFUStrategy<K>
where
    K: Clone + Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            frequency_map: HashMap::new(),
            frequency_tree: BTreeMap::new(),
        }
    }
}

impl<K> EvictionStrategy<K> for LFUStrategy<K>
where
    K: Clone + Eq + Hash,
{
    fn add(&mut self, key: K) {
        self.frequency_map.insert(key.clone(), 1);
        self.frequency_tree
            .entry(1)
            .or_insert_with(Vec::new)
            .push(key);
    }

    fn update(&mut self, key: &K) -> bool {
        if let Some(&old_freq) = self.frequency_map.get(key) {
            // Remove from old frequency
            if let Some(keys) = self.frequency_tree.get_mut(&old_freq) {
                keys.retain(|k| k != key);
                if keys.is_empty() {
                    self.frequency_tree.remove(&old_freq);
                }
            }

            // Add to new frequency
            let new_freq = old_freq + 1;
            self.frequency_map.insert(key.clone(), new_freq);
            self.frequency_tree
                .entry(new_freq)
                .or_insert_with(Vec::new)
                .push(key.clone());
            true
        } else {
            false
        }
    }

    fn evict(&mut self) -> Option<K> {
        if let Some((&min_freq, keys)) = self.frequency_tree.first_key_value() {
            if let Some(key) = keys.first().cloned() {
                // Remove from frequency tree
                if keys.len() == 1 {
                    self.frequency_tree.remove(&min_freq);
                } else {
                    self.frequency_tree.get_mut(&min_freq).unwrap().remove(0);
                }

                // Remove from frequency map
                self.frequency_map.remove(&key);
                Some(key)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn remove(&mut self, key: &K) -> bool {
        if let Some(&freq) = self.frequency_map.get(key) {
            self.frequency_map.remove(key);
            if let Some(keys) = self.frequency_tree.get_mut(&freq) {
                keys.retain(|k| k != key);
                if keys.is_empty() {
                    self.frequency_tree.remove(&freq);
                }
            }
            true
        } else {
            false
        }
    }

    fn len(&self) -> usize {
        self.frequency_map.len()
    }
}

/// First In, First Out (FIFO) eviction strategy
pub struct FIFOStrategy<K> {
    queue: VecDeque<K>,
}

impl<K> FIFOStrategy<K> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl<K: Clone + Eq> EvictionStrategy<K> for FIFOStrategy<K> {
    fn add(&mut self, key: K) {
        self.queue.push_back(key);
    }

    fn update(&mut self, _key: &K) -> bool {
        // FIFO doesn't update access patterns
        false
    }

    fn evict(&mut self) -> Option<K> {
        self.queue.pop_front()
    }

    fn remove(&mut self, key: &K) -> bool {
        if let Some(pos) = self.queue.iter().position(|k| k == key) {
            self.queue.remove(pos);
            true
        } else {
            false
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

/// Clock (Second Chance) eviction strategy
pub struct ClockStrategy<K> {
    entries: Vec<ClockEntry<K>>,
    hand: usize,
}

struct ClockEntry<K> {
    key: K,
    reference_bit: bool,
}

impl<K> ClockStrategy<K>
where
    K: Clone + Eq,
{
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            hand: 0,
        }
    }
}

impl<K> EvictionStrategy<K> for ClockStrategy<K>
where
    K: Clone + Eq,
{
    fn add(&mut self, key: K) {
        self.entries.push(ClockEntry {
            key,
            reference_bit: true,
        });
    }

    fn update(&mut self, key: &K) -> bool {
        for entry in &mut self.entries {
            if entry.key == *key {
                entry.reference_bit = true;
                return true;
            }
        }
        false
    }

    fn evict(&mut self) -> Option<K> {
        if self.entries.is_empty() {
            return None;
        }

        loop {
            let entry = &mut self.entries[self.hand];

            if entry.reference_bit {
                // Give second chance
                entry.reference_bit = false;
                self.hand = (self.hand + 1) % self.entries.len();
            } else {
                // Evict this entry
                let key = entry.key.clone();
                self.entries.remove(self.hand);
                if self.hand >= self.entries.len() {
                    self.hand = 0;
                }
                return Some(key);
            }
        }
    }

    fn remove(&mut self, key: &K) -> bool {
        if let Some(pos) = self.entries.iter().position(|entry| entry.key == *key) {
            self.entries.remove(pos);
            if self.hand >= self.entries.len() {
                self.hand = 0;
            }
            true
        } else {
            false
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Adaptive Replacement Cache (ARC) eviction strategy
pub struct ARCStrategy<K> {
    t1: VecDeque<K>, // Recently accessed items
    t2: VecDeque<K>, // Frequently accessed items
    b1: VecDeque<K>, // Ghost entries for t1
    b2: VecDeque<K>, // Ghost entries for t2
    p: usize,        // Target size for t1
    capacity: usize,
}

impl<K> ARCStrategy<K>
where
    K: Clone + Eq + Hash,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            t1: VecDeque::new(),
            t2: VecDeque::new(),
            b1: VecDeque::new(),
            b2: VecDeque::new(),
            p: 0,
            capacity,
        }
    }

    #[allow(dead_code)] // Planned for future ARC eviction logic
    fn replace(&mut self, _key: K) {
        if !self.t1.is_empty()
            && (self.t1.len() > self.p || (!self.b2.is_empty() && self.t1.len() == self.p))
        {
            // Move from t1 to b1
            if let Some(k) = self.t1.pop_back() {
                self.b1.push_front(k);
            }
        } else {
            // Move from t2 to b2
            if let Some(k) = self.t2.pop_back() {
                self.b2.push_front(k);
            }
        }
    }
}

impl<K> EvictionStrategy<K> for ARCStrategy<K>
where
    K: Clone + Eq,
{
    fn add(&mut self, key: K) {
        if self.t1.len() + self.t2.len() < self.capacity {
            // Cache not full, add to t1
            self.t1.push_front(key);
        } else {
            // Cache full, need to evict
            if self.t1.len() >= self.p {
                // t1 is too large, evict from t1
                if let Some(evicted) = self.t1.pop_back() {
                    self.b1.push_front(evicted);
                    self.t1.push_front(key);
                }
            } else {
                // t2 is too large, evict from t2
                if let Some(evicted) = self.t2.pop_back() {
                    self.b2.push_front(evicted);
                    self.t2.push_front(key);
                }
            }
        }
    }

    fn update(&mut self, key: &K) -> bool {
        // Check t1
        if let Some(pos) = self.t1.iter().position(|k| k == key) {
            let k = self.t1.remove(pos).unwrap();
            self.t2.push_front(k);
            return true;
        }

        // Check t2
        if let Some(pos) = self.t2.iter().position(|k| k == key) {
            let k = self.t2.remove(pos).unwrap();
            self.t2.push_front(k);
            return true;
        }

        false
    }

    fn evict(&mut self) -> Option<K> {
        if !self.t1.is_empty() {
            self.t1.pop_back()
        } else if !self.t2.is_empty() {
            self.t2.pop_back()
        } else {
            None
        }
    }

    fn remove(&mut self, key: &K) -> bool {
        // Remove from t1
        if let Some(pos) = self.t1.iter().position(|k| k == key) {
            self.t1.remove(pos);
            return true;
        }

        // Remove from t2
        if let Some(pos) = self.t2.iter().position(|k| k == key) {
            self.t2.remove(pos);
            return true;
        }

        // Remove from b1
        if let Some(pos) = self.b1.iter().position(|k| k == key) {
            self.b1.remove(pos);
            return true;
        }

        // Remove from b2
        if let Some(pos) = self.b2.iter().position(|k| k == key) {
            self.b2.remove(pos);
            return true;
        }

        false
    }

    fn len(&self) -> usize {
        self.t1.len() + self.t2.len()
    }
}

/// Random eviction strategy
pub struct RandomStrategy<K> {
    keys: Vec<K>,
}

impl<K> RandomStrategy<K> {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }
}

impl<K> EvictionStrategy<K> for RandomStrategy<K>
where
    K: Clone + Eq,
{
    fn add(&mut self, key: K) {
        self.keys.push(key);
    }

    fn update(&mut self, _key: &K) -> bool {
        // Random strategy doesn't track access patterns
        false
    }

    fn evict(&mut self) -> Option<K> {
        if self.keys.is_empty() {
            return None;
        }

        let index = fastrand::usize(..self.keys.len());
        Some(self.keys.remove(index))
    }

    fn remove(&mut self, key: &K) -> bool {
        if let Some(pos) = self.keys.iter().position(|k| k == key) {
            self.keys.remove(pos);
            true
        } else {
            false
        }
    }

    fn len(&self) -> usize {
        self.keys.len()
    }
}

/// Time-based eviction strategy
pub struct TTLStrategy<K> {
    entries: BTreeMap<Instant, Vec<K>>,
    key_to_expiry: HashMap<K, Instant>,
}

impl<K> TTLStrategy<K>
where
    K: Clone + Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            key_to_expiry: HashMap::new(),
        }
    }

    pub fn add_with_ttl(&mut self, key: K, ttl: Duration) {
        let expiry = Instant::now() + ttl;
        self.entries
            .entry(expiry)
            .or_insert_with(Vec::new)
            .push(key.clone());
        self.key_to_expiry.insert(key, expiry);
    }

    pub fn evict_expired(&mut self) -> Vec<K> {
        let now = Instant::now();
        let mut expired = Vec::new();

        let expired_entries: Vec<_> = self
            .entries
            .range(..now)
            .map(|(&expiry, keys)| (expiry, keys.clone()))
            .collect();

        for (expiry, keys) in expired_entries {
            expired.extend(keys.clone());
            self.entries.remove(&expiry);
            for key in keys {
                self.key_to_expiry.remove(&key);
            }
        }

        expired
    }
}

impl<K> EvictionStrategy<K> for TTLStrategy<K>
where
    K: Clone + Eq + Hash,
{
    fn add(&mut self, key: K) {
        // Default TTL of 1 hour
        self.add_with_ttl(key, Duration::from_secs(3600));
    }

    fn update(&mut self, _key: &K) -> bool {
        // TTL strategy doesn't update access patterns
        false
    }

    fn evict(&mut self) -> Option<K> {
        // Evict the earliest expiring key
        if let Some((&_expiry, keys)) = self.entries.first_key_value() {
            if let Some(key) = keys.first().cloned() {
                // Remove from strategy
                self.remove(&key);
                Some(key)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn remove(&mut self, key: &K) -> bool {
        if let Some(expiry) = self.key_to_expiry.remove(key) {
            if let Some(keys) = self.entries.get_mut(&expiry) {
                keys.retain(|k| k != key);
                if keys.is_empty() {
                    self.entries.remove(&expiry);
                }
            }
            true
        } else {
            false
        }
    }

    fn len(&self) -> usize {
        self.key_to_expiry.len()
    }
}
