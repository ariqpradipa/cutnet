//! Device session history tracking
//!
//! Records when devices join and leave the network, persisting to
//! `~/.cutnet/history.json` for survival across restarts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::network::{Device, NetworkError};

/// A single session entry for a device on the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSession {
    pub ip: String,
    pub mac: String,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub custom_name: Option<String>,
    pub joined_at: u64,
    pub left_at: Option<u64>,
}

/// Internal storage wrapper for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryData {
    sessions: Vec<DeviceSession>,
}

static HISTORY: Lazy<Arc<RwLock<HistoryManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HistoryManager::new()))
});

pub struct HistoryManager {
    sessions: Vec<DeviceSession>,
    /// Tracks the currently-online devices keyed by IP so we can
    /// find the *active* session when a device leaves.
    active_sessions: HashMap<String, usize>,
    config_path: PathBuf,
}

impl HistoryManager {
    fn new() -> Self {
        let config_path = std::env::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cutnet");

        fs::create_dir_all(&config_path).ok();
        let config_path = config_path.join("history.json");

        let mut manager = Self {
            sessions: Vec::new(),
            active_sessions: HashMap::new(),
            config_path,
        };

        manager.load();
        manager
    }

    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            if let Ok(data) = serde_json::from_str::<HistoryData>(&content) {
                self.sessions = data.sessions;
                for (idx, session) in self.sessions.iter().enumerate() {
                    if session.left_at.is_none() {
                        self.active_sessions.insert(session.ip.clone(), idx);
                    }
                }
            }
        }
    }

    fn save(&self) -> Result<(), NetworkError> {
        let data = HistoryData {
            sessions: self.sessions.clone(),
        };
        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| NetworkError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        fs::write(&self.config_path, content)
            .map_err(|e| NetworkError::IoError(e))?;
        Ok(())
    }

    /// Log that a device has been discovered (joined the network).
    /// If the device already has an active session, this is a no-op
    /// (the device is already considered online).
    pub async fn log_device_joined(&mut self, device: &Device) {
        // Skip if already active
        if self.active_sessions.contains_key(&device.ip) {
            return;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let session = DeviceSession {
            ip: device.ip.clone(),
            mac: device.mac.clone(),
            hostname: device.hostname.clone(),
            vendor: device.vendor.clone(),
            custom_name: None,
            joined_at: now,
            left_at: None,
        };

        let idx = self.sessions.len();
        self.active_sessions.insert(device.ip.clone(), idx);
        self.sessions.push(session);

        // Persist after each change
        let _ = self.save();
    }

    /// Log that a device has left the network.
    /// Closes the active session for the given IP.
    pub async fn log_device_left(&mut self, ip: &str) {
        if let Some(&idx) = self.active_sessions.get(ip) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if let Some(session) = self.sessions.get_mut(idx) {
                session.left_at = Some(now);
            }
            self.active_sessions.remove(ip);

            let _ = self.save();
        }
    }

    pub async fn get_sessions(&self) -> Vec<DeviceSession> {
        self.sessions.clone()
    }

    pub async fn clear(&mut self) {
        self.sessions.clear();
        self.active_sessions.clear();
        let _ = self.save();
    }
}

// ── Public API ──────────────────────────────────────────────────────

pub async fn log_device_joined(device: &Device) {
    let mut mgr = HISTORY.write().await;
    mgr.log_device_joined(device).await;
}

pub async fn log_device_left(ip: &str) {
    let mut mgr = HISTORY.write().await;
    mgr.log_device_left(ip).await;
}

pub async fn get_sessions() -> Vec<DeviceSession> {
    let mgr = HISTORY.read().await;
    mgr.get_sessions().await
}

pub async fn clear_history() {
    let mut mgr = HISTORY.write().await;
    mgr.clear().await;
}
