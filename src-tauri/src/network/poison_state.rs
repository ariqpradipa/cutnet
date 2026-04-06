//! Crash recovery for ARP poisoning operations.
//!
//! Saves active targets to ~/.cutnet/poisoning_state.json.
//! On app startup, restores and sends ARP replies to unpoison stale targets.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::network::types::{Device, NetworkError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoisonTargetEntry {
    ip: String,
    mac: String,
    router_ip: String,
    router_mac: String,
    interface: String,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PoisonStateData {
    targets: HashMap<String, PoisonTargetEntry>,
}

impl Default for PoisonStateData {
    fn default() -> Self {
        Self {
            targets: HashMap::new(),
        }
    }
}

pub struct PoisonStateManager {
    state_file: PathBuf,
}

impl PoisonStateManager {
    fn new() -> Self {
        let state_file = Self::get_state_file_path();
        Self { state_file }
    }

    fn get_state_file_path() -> PathBuf {
        std::env::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cutnet")
            .join("poisoning_state.json")
    }

    fn ensure_config_dir(&self) -> Result<()> {
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                log::error!("Failed to create config directory: {}", e);
                NetworkError::IoError(e)
            })?;
        }
        Ok(())
    }

    fn load(&self) -> PoisonStateData {
        match fs::read_to_string(&self.state_file) {
            Ok(content) => {
                match serde_json::from_str::<PoisonStateData>(&content) {
                    Ok(data) => data,
                    Err(e) => {
                        log::error!("Failed to parse poisoning state file: {}", e);
                        PoisonStateData::default()
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                PoisonStateData::default()
            }
            Err(e) => {
                log::error!("Failed to read poisoning state file: {}", e);
                PoisonStateData::default()
            }
        }
    }

    fn save(&self, data: &PoisonStateData) -> Result<()> {
        self.ensure_config_dir()?;

        let content = serde_json::to_string_pretty(data).map_err(|e| {
            log::error!("Failed to serialize poisoning state: {}", e);
            NetworkError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        fs::write(&self.state_file, content).map_err(|e| {
            log::error!("Failed to write poisoning state file: {}", e);
            NetworkError::IoError(e)
        })?;

        Ok(())
    }

    fn delete(&self) {
        if self.state_file.exists() {
            if let Err(e) = fs::remove_file(&self.state_file) {
                log::warn!("Failed to delete poisoning state file: {}", e);
            } else {
                log::info!("Poisoning state file deleted");
            }
        }
    }

    fn add_target(
        &self,
        target: &Device,
        router: &Device,
        interface: &str,
    ) -> Result<()> {
        let mut data = self.load();
        let state_key = format!("{}-{}", target.mac, router.mac);

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = PoisonTargetEntry {
            ip: target.ip.clone(),
            mac: target.mac.clone(),
            router_ip: router.ip.clone(),
            router_mac: router.mac.clone(),
            interface: interface.to_string(),
            timestamp,
        };

        data.targets.insert(state_key, entry);
        self.save(&data)?;

        log::info!(
            "Added poisoning target to state: {} ({})",
            target.ip,
            target.mac
        );

        Ok(())
    }

    fn remove_target(&self, target_ip: &str, router_ip: &str) -> Result<()> {
        let mut data = self.load();
        let state_key = format!("{}-{}", target_ip, router_ip);

        if data.targets.remove(&state_key).is_some() {
            if data.targets.is_empty() {
                self.delete();
            } else {
                self.save(&data)?;
            }

            log::info!("Removed poisoning target from state: {}-{}", target_ip, router_ip);
        }

        Ok(())
    }

    fn get_targets(&self) -> Vec<PoisonTargetEntry> {
        let data = self.load();
        data.targets.values().cloned().collect()
    }

    fn target_count(&self) -> usize {
        let data = self.load();
        data.targets.len()
    }
}

static STATE_MANAGER: Lazy<RwLock<PoisonStateManager>> =
    Lazy::new(|| RwLock::new(PoisonStateManager::new()));

pub async fn add_poisoning_target(
    target: &Device,
    router: &Device,
    interface: &str,
) -> Result<()> {
    let manager = STATE_MANAGER.read().await;
    if let Err(e) = manager.add_target(target, router, interface) {
        log::error!("Failed to persist poisoning state: {}", e);
    }
    Ok(())
}

pub async fn remove_poisoning_target(target_ip: &str, router_ip: &str) -> Result<()> {
    let manager = STATE_MANAGER.read().await;
    if let Err(e) = manager.remove_target(target_ip, router_ip) {
        log::error!("Failed to update poisoning state: {}", e);
    }
    Ok(())
}

pub async fn get_active_targets() -> Vec<(Device, Device, String)> {
    let manager = STATE_MANAGER.read().await;
    manager
        .get_targets()
        .into_iter()
        .map(|entry| {
            let target = Device::new(entry.ip, entry.mac);
            let router = Device::new(entry.router_ip, entry.router_mac);
            (target, router, entry.interface)
        })
        .collect()
}

pub async fn has_active_targets() -> bool {
    let manager = STATE_MANAGER.read().await;
    manager.target_count() > 0
}

pub async fn clear_all_state() {
    let manager = STATE_MANAGER.read().await;
    manager.delete();
}

pub async fn recover_from_crash() -> Result<usize> {
    let targets = get_active_targets().await;
    let total_targets = targets.len();

    if total_targets == 0 {
        log::info!("No previous poisoning state found");
        return Ok(0);
    }

    log::info!(
        "Crash recovery: Found {} poisoned device(s)",
        total_targets
    );

    let mut restored_count = 0;

    for (target, router, interface) in targets {
        log::info!(
            "Crash recovery: Restoring {} ({})",
            target.ip, target.mac
        );

        match send_restore_packets(&target, &router, &interface).await {
            Ok(_) => {
                log::info!("Successfully restored device {}", target.ip);
                restored_count += 1;
            }
            Err(e) => {
                log::error!("Failed to restore device {}: {}", target.ip, e);
            }
        }
    }

    clear_all_state().await;

    log::info!(
        "Crash recovery complete: {} of {} device(s) restored",
        restored_count,
        total_targets
    );

    Ok(restored_count)
}

async fn send_restore_packets(
    target: &Device,
    router: &Device,
    interface_name: &str,
) -> Result<()> {
    use crate::network::poisoner::send_single_restore;

    send_single_restore(target, router, interface_name).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_manager_basic() {
        let temp_file = std::env::temp_dir().join("test_poison_state.json");
        let manager = PoisonStateManager {
            state_file: temp_file.clone(),
        };

        let _ = fs::remove_file(&temp_file);

        assert_eq!(manager.target_count(), 0);

        let target = Device::new("192.168.1.100", "aa:bb:cc:dd:ee:ff");
        let router = Device::new("192.168.1.1", "11:22:33:44:55:66");
        manager.add_target(&target, &router, "eth0").unwrap();

        assert_eq!(manager.target_count(), 1);

        let target2 = Device::new("192.168.1.101", "aa:bb:cc:dd:ee:00");
        manager.add_target(&target2, &router, "eth0").unwrap();

        assert_eq!(manager.target_count(), 2);

        manager.remove_target(&target.ip, &router.ip).unwrap();
        assert_eq!(manager.target_count(), 1);

        manager.remove_target(&target2.ip, &router.ip).unwrap();
        assert_eq!(manager.target_count(), 0);
        assert!(!temp_file.exists());

        let _ = fs::remove_file(&temp_file);
    }

    #[test]
    fn test_state_file_path() {
        let manager = PoisonStateManager::new();
        let path = manager.state_file;
        assert!(path.to_string_lossy().contains(".cutnet"));
        assert!(path.to_string_lossy().contains("poisoning_state.json"));
    }
}
