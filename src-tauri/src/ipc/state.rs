//! State management for IPC commands
//!
//! This module manages shared state between Tauri commands using
//! tokio::sync::Mutex for thread-safe access.

use crate::ipc::events::{emit_scan_progress, emit_scan_completed};
use crate::network::{Device, NetworkError, PoisoningConfig, PoisoningState};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

/// Global killer state - manages ARP poisoning operations
pub type KillerState = Arc<Mutex<Killer>>;

/// Global scanner state - manages network scanning operations
pub type ScannerState = Arc<Mutex<Scanner>>;

/// Manages ARP poisoning (device killing) operations
#[derive(Debug)]
pub struct Killer {
    /// Currently poisoned devices (ip -> mac mapping)
    poisoned_devices: HashMap<String, String>,
    /// Current poisoning state
    state: PoisoningState,
    /// Configuration for poisoning operations
    config: PoisoningConfig,
    /// Flag to signal stop
    should_stop: bool,
}

impl Killer {
    /// Create a new Killer instance
    pub fn new() -> Self {
        Self {
            poisoned_devices: HashMap::new(),
            state: PoisoningState::Idle,
            config: PoisoningConfig::default(),
            should_stop: false,
        }
    }

    /// Start poisoning a device
    pub fn kill_device(&mut self, ip: String, mac: String) -> Result<(), NetworkError> {
        if self.poisoned_devices.contains_key(&ip) {
            return Err(NetworkError::PoisoningError(format!(
                "Device {} is already being poisoned",
                ip
            )));
        }

        self.poisoned_devices.insert(ip.clone(), mac.clone());
        self.state = PoisoningState::Active;
        
        log::info!("Started poisoning device {} ({})", ip, mac);
        
        // In a real implementation, this would start a background task
        // to send ARP packets at regular intervals
        
        Ok(())
    }

    /// Stop poisoning a device and send restore packets
    pub fn unkill_device(&mut self, ip: String, mac: String) -> Result<(), NetworkError> {
        if !self.poisoned_devices.contains_key(&ip) {
            return Err(NetworkError::PoisoningError(format!(
                "Device {} is not being poisoned",
                ip
            )));
        }

        self.poisoned_devices.remove(&ip);
        
        // Send restore packets
        log::info!("Sending restore packets for {} ({})", ip, mac);
        
        // If no more devices are poisoned, set state to idle
        if self.poisoned_devices.is_empty() {
            self.state = PoisoningState::Idle;
        }
        
        Ok(())
    }

    /// Stop poisoning all devices
    pub fn unkill_all(&mut self) -> Result<Vec<(String, String)>, NetworkError> {
        let devices: Vec<(String, String)> = self
            .poisoned_devices
            .drain()
            .collect();
        
        // Send restore packets for all devices
        for (ip, mac) in &devices {
            log::info!("Restoring device {} ({})", ip, mac);
        }
        
        self.state = PoisoningState::Idle;
        
        Ok(devices)
    }

    /// Get list of poisoned devices
    pub fn get_poisoned_devices(&self) -> Vec<(String, String)> {
        self.poisoned_devices
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check if a specific device is being poisoned
    pub fn is_poisoned(&self, ip: &str) -> bool {
        self.poisoned_devices.contains_key(ip)
    }

    /// Get current poisoning state
    pub fn get_state(&self) -> PoisoningState {
        self.state
    }

    /// Signal the killer to stop
    pub fn stop(&mut self) {
        self.should_stop = true;
    }

    /// Check if stop has been signaled
    pub fn should_stop(&self) -> bool {
        self.should_stop
    }
}

impl Default for Killer {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages network scanning operations
#[derive(Debug)]
pub struct Scanner {
    /// Whether a scan is currently running
    is_running: bool,
    /// Discovered devices during current scan
    discovered_devices: Vec<Device>,
    /// Interface being scanned
    current_interface: Option<String>,
    /// Progress percentage (0-100)
    progress: u8,
    /// Flag to signal stop
    should_stop: bool,
}

impl Scanner {
    /// Create a new Scanner instance
    pub fn new() -> Self {
        Self {
            is_running: false,
            discovered_devices: Vec::new(),
            current_interface: None,
            progress: 0,
            should_stop: false,
        }
    }

    /// Start an ARP scan on the specified interface
    pub fn start_arp_scan(
        &mut self,
        interface_name: String,
        app: AppHandle,
    ) -> Result<(), NetworkError> {
        if self.is_running {
            return Err(NetworkError::ArpScanError(
                "Scan already in progress".to_string(),
            ));
        }

        log::info!("Starting ARP scan on interface: {}", interface_name);

        self.is_running = true;
        self.discovered_devices.clear();
        self.current_interface = Some(interface_name);
        self.progress = 0;
        self.should_stop = false;

        // In a real implementation, this would spawn an async task
        // that performs the actual ARP scanning and emits events
        // For now, we'll simulate the scan
        tokio::spawn(async move {
            // Simulate scan progress
            for i in 0..=100 {
                if i % 10 == 0 {
                    emit_scan_progress(&app, i, 0);
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
            emit_scan_completed(&app, 0, true);
        });

        Ok(())
    }

    /// Start a ping scan on the specified interface
    pub fn start_ping_scan(
        &mut self,
        interface_name: String,
        app: AppHandle,
    ) -> Result<(), NetworkError> {
        if self.is_running {
            return Err(NetworkError::PingScanError(
                "Scan already in progress".to_string(),
            ));
        }

        self.is_running = true;
        self.discovered_devices.clear();
        self.current_interface = Some(interface_name.clone());
        self.progress = 0;
        self.should_stop = false;

        log::info!("Starting ping scan on interface: {}", interface_name);

        // In a real implementation, this would spawn an async task
        // that performs ICMP ping scanning
        tokio::spawn(async move {
            // Simulate scan progress
            for i in 0..=100 {
                if i % 10 == 0 {
                    emit_scan_progress(&app, i, 0);
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
            }
            emit_scan_completed(&app, 0, true);
        });

        Ok(())
    }

    /// Stop the current scan
    pub fn stop_scan(&mut self) {
        if self.is_running {
            self.should_stop = true;
            self.is_running = false;
            log::info!("Scan stopped");
        }
    }

    /// Add a discovered device
    pub fn add_device(&mut self, device: Device) {
        self.discovered_devices.push(device);
    }

    /// Get discovered devices
    pub fn get_devices(&self) -> &[Device] {
        &self.discovered_devices
    }

    /// Check if a scan is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get current progress
    pub fn get_progress(&self) -> u8 {
        self.progress
    }

    /// Update progress
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = progress.min(100);
    }

    /// Check if stop has been signaled
    pub fn should_stop(&self) -> bool {
        self.should_stop
    }

    /// Get the current interface being scanned
    pub fn get_current_interface(&self) -> Option<&String> {
        self.current_interface.as_ref()
    }

    /// Clear discovered devices
    pub fn clear_devices(&mut self) {
        self.discovered_devices.clear();
    }
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize global state
pub fn init_state() -> (KillerState, ScannerState) {
    let killer = Arc::new(Mutex::new(Killer::new()));
    let scanner = Arc::new(Mutex::new(Scanner::new()));
    
    log::info!("IPC state initialized");
    
    (killer, scanner)
}
