use std::{collections::HashMap, time::Instant};

pub struct Store {
    // Fields for the store, such as data structures to hold key-value pairs, etc.
    data_store: HashMap<String, String>,
    // Can be empty as well, in case key doesn't have timestamp. Timestamps are better stored as Intants than u64 or any other type 
    time_store: HashMap<String, Instant>, 
}

impl Store {
    pub fn new() -> Self {
        Self {
            data_store: HashMap::new(),
            time_store: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.data_store.get(key)
    }

    pub fn get_ttl(&self, key: &str) -> Option<&Instant> {
        self.time_store.get(key)
    }

    pub fn set(&mut self, key: String, value: String) {
        self.data_store.insert(key, value);

    }
}
