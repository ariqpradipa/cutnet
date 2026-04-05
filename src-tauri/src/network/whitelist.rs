use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::network::NetworkError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistEntry {
    pub mac: String,
    pub label: Option<String>,
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WhitelistData {
    entries: HashMap<String, WhitelistEntry>,
    protect_enabled: bool,
}

static WHITELIST: Lazy<Arc<RwLock<WhitelistManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(WhitelistManager::new()))
});

pub struct WhitelistManager {
    entries: HashMap<String, WhitelistEntry>,
    protect_enabled: bool,
    config_path: PathBuf,
}

impl WhitelistManager {
    fn new() -> Self {
        let config_path = std::env::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cutnet");
        
        fs::create_dir_all(&config_path).ok();
        let config_path = config_path.join("whitelist.json");
        
        let mut manager = Self {
            entries: HashMap::new(),
            protect_enabled: true,
            config_path,
        };
        
        manager.load();
        manager
    }

    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            if let Ok(data) = serde_json::from_str::<WhitelistData>(&content) {
                self.entries = data.entries;
                self.protect_enabled = data.protect_enabled;
            }
        }
    }

    fn save(&self) -> Result<(), NetworkError> {
        let data = WhitelistData {
            entries: self.entries.clone(),
            protect_enabled: self.protect_enabled,
        };
        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| NetworkError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        fs::write(&self.config_path, content)
            .map_err(|e| NetworkError::IoError(e))?;
        Ok(())
    }

    pub async fn add_entry(&mut self, mac: String, label: Option<String>) -> Result<(), NetworkError> {
        let mac_lower = mac.to_lowercase();
        let entry = WhitelistEntry {
            mac: mac_lower.clone(),
            label,
            added_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        self.entries.insert(mac_lower, entry);
        self.save()
    }

    pub async fn remove_entry(&mut self, mac: &str) -> bool {
        let removed = self.entries.remove(&mac.to_lowercase()).is_some();
        if removed {
            let _ = self.save();
        }
        removed
    }

    pub async fn is_whitelisted(&self, mac: &str) -> bool {
        self.entries.contains_key(&mac.to_lowercase())
    }

    pub async fn get_entries(&self) -> Vec<WhitelistEntry> {
        self.entries.values().cloned().collect()
    }

    pub async fn set_protect_enabled(&mut self, enabled: bool) -> Result<(), NetworkError> {
        self.protect_enabled = enabled;
        self.save()
    }

    #[allow(dead_code)]
    pub async fn is_protected(&self, mac: &str) -> bool {
        self.protect_enabled && self.entries.contains_key(&mac.to_lowercase())
    }
}

// Public API functions
pub async fn add_entry(mac: String, label: Option<String>) -> Result<(), NetworkError> {
    let mut wl = WHITELIST.write().await;
    wl.add_entry(mac, label).await
}

pub async fn remove_entry(mac: &str) -> bool {
    let mut wl = WHITELIST.write().await;
    wl.remove_entry(mac).await
}

pub async fn is_whitelisted(mac: &str) -> bool {
    let wl = WHITELIST.read().await;
    wl.is_whitelisted(mac).await
}

pub async fn get_entries() -> Vec<WhitelistEntry> {
    let wl = WHITELIST.read().await;
    wl.get_entries().await
}

pub async fn set_protect_enabled(enabled: bool) -> Result<(), NetworkError> {
    let mut wl = WHITELIST.write().await;
    wl.set_protect_enabled(enabled).await
}

#[allow(dead_code)]
pub async fn is_protected(mac: &str) -> bool {
    let wl = WHITELIST.read().await;
    wl.is_protected(mac).await
}
