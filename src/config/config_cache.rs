use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{path, time};

/// A struct to store the hash and modified time of a file
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct CacheEntry {
    /// The hash of the entry
    pub hash: String,

    /// The modified time of the entry
    pub modified: time::SystemTime,
}

/// A serializable version of DashMap
#[derive(Serialize, Deserialize, Default)]
pub struct ConfigCacheSerde {
    inner: Vec<(path::PathBuf, CacheEntry)>,
}

impl ConfigCacheSerde {
    /// Convert DashMap to a serializable ConfigCacheSerde
    pub fn from_dashmap(dashmap: &DashMap<path::PathBuf, CacheEntry>) -> Self {
        let inner = dashmap
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        ConfigCacheSerde { inner }
    }

    /// Convert ConfigCacheSerde back to DashMap
    pub fn to_dashmap(self) -> DashMap<path::PathBuf, CacheEntry> {
        let dashmap = DashMap::new();
        for (key, value) in self.inner {
            dashmap.insert(key, value);
        }
        dashmap
    }
}
