//! Device session history tracking
//!
//! Records when devices join and leave the network, persisting to
//! `~/.cutnet/history.json` for survival across restarts.
//!
//! Disk writes are batched: individual join/leave calls mark the state as
//! dirty; the actual write is deferred to `flush()` which is called once
//! at scan completion, avoiding one fsync per device on large scans.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::network::{Device, NetworkError};

/// A single session entry for a device on the network.
///
/// Field names are deliberately kept consistent with the TypeScript
/// `HistoryEntry` schema in `src/lib/schemas.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSession {
    pub ip: String,
    pub mac: String,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub custom_name: Option<String>,
    /// UNIX timestamp (seconds) when the device joined
    pub join_time: u64,
    /// UNIX timestamp (seconds) when the device left, or null if still online
    pub leave_time: Option<u64>,
    /// Whether this device was killed (ARP-poisoned) during this session
    pub was_killed: bool,
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
    /// True when in-memory state differs from what was last written to disk.
    dirty: bool,
}

impl HistoryManager {
    fn new() -> Self {
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cutnet");

        std::fs::create_dir_all(&config_path).ok();
        let config_path = config_path.join("history.json");

        let mut manager = Self {
            sessions: Vec::new(),
            active_sessions: HashMap::new(),
            config_path,
            dirty: false,
        };

        manager.load();
        manager
    }

    fn load(&mut self) {
        if let Ok(content) = std::fs::read_to_string(&self.config_path) {
            if let Ok(data) = serde_json::from_str::<HistoryData>(&content) {
                self.sessions = data.sessions;
                for (idx, session) in self.sessions.iter().enumerate() {
                    if session.leave_time.is_none() {
                        self.active_sessions.insert(session.ip.clone(), idx);
                    }
                }
            }
        }
    }

    /// Write to disk asynchronously — called explicitly at scan completion.
    async fn flush(&mut self) -> Result<(), NetworkError> {
        if !self.dirty {
            return Ok(());
        }
        let data = HistoryData {
            sessions: self.sessions.clone(),
        };
        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| NetworkError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        let path = self.config_path.clone();
        // Use spawn_blocking so we never block the tokio executor (#24)
        tokio::task::spawn_blocking(move || std::fs::write(&path, content))
            .await
            .map_err(|e| NetworkError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
            .map_err(NetworkError::IoError)?;
        self.dirty = false;
        Ok(())
    }

    /// Log that a device has been discovered (joined the network).
    /// Marks state dirty but does NOT flush to disk — call `flush()` after
    /// processing a full scan batch.
    pub fn log_device_joined(&mut self, device: &Device) {
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
            join_time: now,
            leave_time: None,
            was_killed: false,
        };

        let idx = self.sessions.len();
        self.active_sessions.insert(device.ip.clone(), idx);
        self.sessions.push(session);
        self.dirty = true;
    }

    /// Log that a device has left the network.
    pub fn log_device_left(&mut self, ip: &str) {
        if let Some(&idx) = self.active_sessions.get(ip) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if let Some(session) = self.sessions.get_mut(idx) {
                session.leave_time = Some(now);
            }
            self.active_sessions.remove(ip);
            self.dirty = true;
        }
    }

    /// Mark a device as having been killed (ARP-poisoned) in its active session.
    pub fn mark_killed(&mut self, ip: &str) {
        if let Some(&idx) = self.active_sessions.get(ip) {
            if let Some(session) = self.sessions.get_mut(idx) {
                session.was_killed = true;
                self.dirty = true;
            }
        }
    }

    pub fn get_sessions(&self) -> Vec<DeviceSession> {
        self.sessions.clone()
    }

    pub async fn clear(&mut self) {
        self.sessions.clear();
        self.active_sessions.clear();
        self.dirty = true;
        let _ = self.flush().await;
    }
}

// ── Public API ──────────────────────────────────────────────────────

pub async fn log_device_joined(device: &Device) {
    let mut mgr = HISTORY.write().await;
    mgr.log_device_joined(device);
    // Do NOT flush here — caller should call flush_history() after a full scan
}

pub async fn log_device_left(ip: &str) {
    let mut mgr = HISTORY.write().await;
    mgr.log_device_left(ip);
}

/// Mark a device's active session as killed.
pub async fn mark_device_killed(ip: &str) {
    let mut mgr = HISTORY.write().await;
    mgr.mark_killed(ip);
}

/// Flush dirty state to disk.  Call once after processing a full scan batch.
pub async fn flush_history() {
    let mut mgr = HISTORY.write().await;
    if let Err(e) = mgr.flush().await {
        log::error!("Failed to flush history: {}", e);
    }
}

pub async fn get_sessions() -> Vec<DeviceSession> {
    let mgr = HISTORY.read().await;
    mgr.get_sessions()
}

pub async fn clear_history() {
    let mut mgr = HISTORY.write().await;
    mgr.clear().await;
}
