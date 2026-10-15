use bytes::Bytes;
use std::collections::HashMap;

pub struct Store {
    // Fields for the store, such as data structures to hold key-value pairs, etc.
    data_store: HashMap<Bytes, Bytes>,
    // Can be empty as well, in case key doesn't have timestamp.
    // We store as u64 due to 2 reasons
    time_store: HashMap<Bytes, u64>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            data_store: HashMap::new(),
            time_store: HashMap::new(),
        }
    }

    pub fn contains(&self, key: &Bytes) -> bool {
        return self.get(key).is_some();
    }

    // Need to add expiry validation
    pub fn get(&mut self, key: &Bytes) -> Option<&Bytes> {
        self.data_store.get(key)
    }

    pub fn get_ttl(&self, key: &Bytes) -> Option<&u64> {
        self.time_store.get(key)
    }

    pub fn set(&mut self, key: Bytes, value: Bytes) {
        self.data_store.insert(key, value);
    }

    pub fn remove_key(&mut self, key: &Bytes) {
        self.data_store.remove(key);
        self.time_store.remove(key);
    }
}
