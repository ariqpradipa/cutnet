//! Persistent MAC-based kill tracking
//!
//! This module manages persistent storage of killed MAC addresses.
//! Unlike the in-memory Killer state which is cleared on app restart,
//! this persistence ensures killed devices remain blocked across restarts.
//! MAC addresses are tracked to prevent DHCP/IP renewal bypass.

#![allow(dead_code)]

use crate::network::types::PersistentKillTarget;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::network::NetworkError;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KilledMacsData {
    macs: HashMap<String, PersistentKillTarget>,
    version: u32,
}

static KILLED_MACS: Lazy<Arc<RwLock<KilledMacsManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(KilledMacsManager::new()))
});

pub struct KilledMacsManager {
    macs: HashMap<String, PersistentKillTarget>,
    config_path: PathBuf,
}

impl KilledMacsManager {
    fn new() -> Self {
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cutnet");

        fs::create_dir_all(&config_path).ok();
        let config_path = config_path.join("killed_macs.json");

        let mut manager = Self {
            macs: HashMap::new(),
            config_path,
        };

        manager.load();
        manager
    }

    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            if let Ok(data) = serde_json::from_str::<KilledMacsData>(&content) {
                self.macs = data.macs;
                log::info!("Loaded {} killed MACs from persistence", self.macs.len());
            }
        }
    }

    fn save(&self) -> Result<(), NetworkError> {
        let data = KilledMacsData {
            macs: self.macs.clone(),
            version: 1,
        };
        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| NetworkError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        fs::write(&self.config_path, content)
            .map_err(|e| NetworkError::IoError(e))?;
        Ok(())
    }

    pub async fn add_mac(&mut self, mac: String, ip: String) -> Result<(), NetworkError> {
        let mac_lower = mac.to_lowercase();
        
        if self.macs.contains_key(&mac_lower) {
            log::debug!("MAC {} already in killed list", mac_lower);
            return Ok(());
        }

        let target = PersistentKillTarget::new(mac_lower.clone(), ip);
        self.macs.insert(mac_lower.clone(), target);
        self.save()?;
        
        log::info!("Added MAC {} to killed list", mac_lower);
        Ok(())
    }

    pub async fn remove_mac(&mut self, mac: &str) -> bool {
        let mac_lower = mac.to_lowercase();
        let removed = self.macs.remove(&mac_lower).is_some();
        if removed {
            let _ = self.save();
            log::info!("Removed MAC {} from killed list", mac_lower);
        }
        removed
    }

    pub async fn is_killed(&self, mac: &str) -> bool {
        self.macs.contains_key(&mac.to_lowercase())
    }

    pub async fn get_all(&self) -> Vec<PersistentKillTarget> {
        self.macs.values().cloned().collect()
    }

    pub async fn clear_all(&mut self) -> Result<(), NetworkError> {
        self.macs.clear();
        self.save()
    }

    pub async fn find_by_mac(&self, mac: &str) -> Option<PersistentKillTarget> {
        self.macs.get(&mac.to_lowercase()).cloned()
    }

    pub async fn update_ip(&mut self, mac: &str, new_ip: String) -> Result<(), NetworkError> {
        let mac_lower = mac.to_lowercase();
        
        if let Some(target) = self.macs.get_mut(&mac_lower) {
            target.first_seen_ip = new_ip;
            self.save()?;
            Ok(())
        } else {
            Err(NetworkError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, format!("MAC {} not found", mac))))
        }
    }
}

// Public API functions

pub async fn add_mac(mac: String, ip: String) -> Result<(), NetworkError> {
    let mut mgr = KILLED_MACS.write().await;
    mgr.add_mac(mac, ip).await
}

pub async fn remove_mac(mac: &str) -> bool {
    let mut mgr = KILLED_MACS.write().await;
    mgr.remove_mac(mac).await
}

pub async fn is_killed(mac: &str) -> bool {
    let mgr = KILLED_MACS.read().await;
    mgr.is_killed(mac).await
}

pub async fn get_all() -> Vec<PersistentKillTarget> {
    let mgr = KILLED_MACS.read().await;
    mgr.get_all().await
}

pub async fn clear_all() -> Result<(), NetworkError> {
    let mut mgr = KILLED_MACS.write().await;
    mgr.clear_all().await
}

pub async fn find_by_mac(mac: &str) -> Option<PersistentKillTarget> {
    let mgr = KILLED_MACS.read().await;
    mgr.find_by_mac(mac).await
}

pub async fn update_ip(mac: &str, new_ip: String) -> Result<(), NetworkError> {
    let mut mgr = KILLED_MACS.write().await;
    mgr.update_ip(mac, new_ip).await
}
