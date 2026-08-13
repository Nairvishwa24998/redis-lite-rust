use std::collections::HashMap;

pub struct Store {
    // Fields for the store, such as data structures to hold key-value pairs, etc.
    data_store: HashMap<String, String>,
    time_store: HashMap<String, u64>,
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

    pub fn get_time_stamp(&self, key: &str) -> Option<&u64> {
        self.time_store.get(key)
    }

    pub fn set(&mut self, key: String, value: String) {
        self.data_store.insert(key, value);
    }
}
