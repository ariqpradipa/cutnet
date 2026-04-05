//! State management for IPC commands
//!
//! This module manages shared state between Tauri commands using
//! tokio::sync::Mutex for thread-safe access.

use crate::ipc::events::{emit_device_found, emit_scan_progress, emit_scan_completed};
use crate::network::{Device, NetworkError};
use crate::network::poisoner::{start_poisoning, stop_poisoning};
use crate::network::whitelist::{is_whitelisted, is_protected};
use crate::network::history;
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
    /// The network interface name to use for poisoning
    interface_name: Option<String>,
    /// The router/gateway device for poisoning
    router: Option<Device>,
}

impl Killer {
    /// Create a new Killer instance
    pub fn new() -> Self {
        Self {
            poisoned_devices: HashMap::new(),
            interface_name: None,
            router: None,
        }
    }

    /// Set the interface name and router device for poisoning operations
    pub fn set_interface_and_router(&mut self, interface: String, router: Device) {
        self.interface_name = Some(interface);
        self.router = Some(router);
    }

    /// Start poisoning a device
    pub async fn kill_device(&mut self, ip: String, mac: String) -> Result<(), NetworkError> {
        if self.poisoned_devices.contains_key(&ip) {
            return Err(NetworkError::PoisoningError(format!(
                "Device {} is already being poisoned",
                ip
            )));
        }

        // CRITICAL SECURITY: Validate MAC address before poisoning
        // Rejects broadcast, multicast, and all-zeros addresses to prevent network-wide DoS
        let _ = crate::network::utils::validate_unicast_mac(&mac)?;

        // Check whitelist - if device is whitelisted and protection is enabled, reject the kill
        if is_whitelisted(&mac).await && is_protected(&mac).await {
            return Err(NetworkError::PoisoningError(
                format!("Device {} ({}) is whitelisted and protected", mac, ip)
            ));
        }

        let interface_name = self.interface_name.clone().ok_or_else(|| {
            NetworkError::PoisoningError(
                "No interface configured. Run a scan first or set the active interface.".to_string(),
            )
        })?;

        let router = self.router.clone().ok_or_else(|| {
            NetworkError::PoisoningError(
                "No router configured. Run a scan first to detect the gateway.".to_string(),
            )
        })?;

        let target = Device::new(&ip, &mac);

        log::info!(
            "Starting poisoning: target={} ({}), router={} ({}), interface={}",
            ip, mac, router.ip, router.mac, interface_name
        );

        start_poisoning(target, router, &interface_name).await?;

        self.poisoned_devices.insert(ip.clone(), mac.clone());

        log::info!("Started poisoning device {} ({})", ip, mac);

        Ok(())
    }

    /// Stop poisoning a device and send restore packets
    pub async fn unkill_device(&mut self, ip: String, mac: String) -> Result<(), NetworkError> {
        if !self.poisoned_devices.contains_key(&ip) {
            return Err(NetworkError::PoisoningError(format!(
                "Device {} is not being poisoned",
                ip
            )));
        }

        let interface_name = self.interface_name.clone().ok_or_else(|| {
            NetworkError::PoisoningError("No interface configured".to_string())
        })?;

        let router = self.router.clone().ok_or_else(|| {
            NetworkError::PoisoningError("No router configured".to_string())
        })?;

        let target = Device::new(&ip, &mac);

        log::info!("Stopping poisoning for {} ({})", ip, mac);

        stop_poisoning(target, router, &interface_name).await?;

        self.poisoned_devices.remove(&ip);

        Ok(())
    }

    /// Stop poisoning all devices
    pub async fn unkill_all(&mut self) -> Result<Vec<(String, String)>, NetworkError> {
        let devices: Vec<(String, String)> = self
            .poisoned_devices
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Stop each device individually
        for (ip, mac) in &devices {
            if let Err(e) = self.unkill_device(ip.clone(), mac.clone()).await {
                log::error!("Failed to stop poisoning {}: {}", ip, e);
            }
        }

        // Clear the map since unkill_device removes entries one by one,
        // but in case of errors, ensure it's clean
        self.poisoned_devices.clear();

        Ok(devices)
    }

    /// Get list of poisoned devices
    #[allow(dead_code)]
    pub fn get_poisoned_devices(&self) -> Vec<(String, String)> {
        self.poisoned_devices
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check if a specific device is being poisoned
    #[allow(dead_code)]
    pub fn is_poisoned(&self, ip: &str) -> bool {
        self.poisoned_devices.contains_key(ip)
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
    /// AppHandle for emitting events from background tasks
    app: Option<AppHandle>,
    /// Track known devices (by IP) to detect joins/leaves for history
    known_devices: HashMap<String, Device>,
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
            app: None,
            known_devices: HashMap::new(),
        }
    }

    /// Start an ARP scan on the specified interface
    pub fn start_arp_scan(
        &mut self,
        interface_name: String,
        app: AppHandle,
        self_arc: ScannerState,
    ) -> Result<(), NetworkError> {
        if self.is_running {
            return Err(NetworkError::ArpScanError(
                "Scan already in progress".to_string(),
            ));
        }

        log::info!("Starting ARP scan on interface: {}", interface_name);

        self.is_running = true;
        self.discovered_devices.clear();
        self.current_interface = Some(interface_name.clone());
        self.progress = 0;
        self.should_stop = false;
        self.app = Some(app.clone());

        let interface_for_scan = interface_name.clone();

        tokio::spawn(async move {
            // Spawn the scan task
            let scan_handle = tokio::spawn(async move {
                crate::network::scanner::arp_scan(&interface_for_scan).await
            });

            // Await the scan result and catch any panics
            let scan_result: Result<
                Result<Vec<crate::network::Device>, crate::network::NetworkError>,
                tokio::task::JoinError,
            > = scan_handle.await;

            // Handle the scan result
            match scan_result {
                Ok(Ok(devices)) => {
                    let total = devices.len() as u16;
                    let mut current_ips = HashMap::new();
                    for device in &devices {
                        current_ips.insert(device.ip.clone(), device.clone());
                    }
                    
                    let known_ips: Vec<String> = {
                        let scanner = self_arc.lock().await;
                        scanner.known_devices.keys().cloned().collect()
                    };
                    for ip in known_ips {
                        if !current_ips.contains_key(&ip) {
                            history::log_device_left(&ip).await;
                            let mut scanner = self_arc.lock().await;
                            scanner.known_devices.remove(&ip);
                        }
                    }
                    
                    for (i, device) in devices.iter().enumerate() {
                        let is_new = {
                            let mut scanner = self_arc.lock().await;
                            let is_new = !scanner.known_devices.contains_key(&device.ip);
                            if is_new {
                                scanner.known_devices.insert(device.ip.clone(), device.clone());
                            }
                            is_new
                        };
                        
                        emit_device_found(&app, device.clone());
                        
                        if is_new {
                            history::log_device_joined(device).await;
                        }
                        
                        let progress = if total > 0 {
                            ((i as f32 / total as f32) * 100.0) as u8
                        } else {
                            0
                        };
                        emit_scan_progress(&app, progress, (i + 1) as u16);
                    }
                    emit_scan_completed(&app, total, true);
                }
                Ok(Err(e)) => {
                    log::error!("ARP scan failed: {}", e);
                    emit_scan_completed(&app, 0, false);
                }
                Err(panic_info) => {
                    log::error!("ARP scan panicked: {:?}", panic_info);
                    emit_scan_completed(&app, 0, false);
                }
            }
            self_arc.lock().await.is_running = false;
        });

        Ok(())
    }

    /// Start a ping scan on the specified interface
    pub fn start_ping_scan(
        &mut self,
        interface_name: String,
        app: AppHandle,
        self_arc: ScannerState,
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
        self.app = Some(app.clone());

        let interface_for_scan = interface_name.clone();

        log::info!("Starting ping scan on interface: {}", interface_name);

        tokio::spawn(async move {
            // Spawn the scan task
            let scan_handle = tokio::spawn(async move {
                crate::network::scanner::ping_scan(&interface_for_scan).await
            });

            // Await the scan result and catch any panics
            let scan_result: Result<
                Result<Vec<crate::network::Device>, crate::network::NetworkError>,
                tokio::task::JoinError,
            > = scan_handle.await;

            // Handle the scan result
            match scan_result {
                Ok(Ok(devices)) => {
                    let total = devices.len() as u16;
                    let mut current_ips = HashMap::new();
                    for device in &devices {
                        current_ips.insert(device.ip.clone(), device.clone());
                    }
                    
                    let known_ips: Vec<String> = {
                        let scanner = self_arc.lock().await;
                        scanner.known_devices.keys().cloned().collect()
                    };
                    for ip in known_ips {
                        if !current_ips.contains_key(&ip) {
                            history::log_device_left(&ip).await;
                            let mut scanner = self_arc.lock().await;
                            scanner.known_devices.remove(&ip);
                        }
                    }
                    
                    for (i, device) in devices.iter().enumerate() {
                        let is_new = {
                            let mut scanner = self_arc.lock().await;
                            let is_new = !scanner.known_devices.contains_key(&device.ip);
                            if is_new {
                                scanner.known_devices.insert(device.ip.clone(), device.clone());
                            }
                            is_new
                        };
                        
                        emit_device_found(&app, device.clone());
                        
                        if is_new {
                            history::log_device_joined(device).await;
                        }
                        
                        let progress = if total > 0 {
                            ((i as f32 / total as f32) * 100.0) as u8
                        } else {
                            0
                        };
                        emit_scan_progress(&app, progress, (i + 1) as u16);
                    }
                    emit_scan_completed(&app, total, true);
                }
                Ok(Err(e)) => {
                    log::error!("Ping scan failed: {}", e);
                    emit_scan_completed(&app, 0, false);
                }
                Err(panic_info) => {
                    log::error!("Ping scan panicked: {:?}", panic_info);
                    emit_scan_completed(&app, 0, false);
                }
            }
            self_arc.lock().await.is_running = false;
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
    #[allow(dead_code)]
    pub fn add_device(&mut self, device: Device) {
        self.discovered_devices.push(device);
    }

    /// Get discovered devices
    #[allow(dead_code)]
    pub fn get_devices(&self) -> &[Device] {
        &self.discovered_devices
    }

    /// Check if a scan is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get current progress
    #[allow(dead_code)]
    pub fn get_progress(&self) -> u8 {
        self.progress
    }

    /// Update progress
    #[allow(dead_code)]
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = progress.min(100);
    }

    /// Check if stop has been signaled
    #[allow(dead_code)]
    pub fn should_stop(&self) -> bool {
        self.should_stop
    }

    /// Get the current interface being scanned
    #[allow(dead_code)]
    pub fn get_current_interface(&self) -> Option<&String> {
        self.current_interface.as_ref()
    }

    /// Clear discovered devices
    #[allow(dead_code)]
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

/// Cleanup all state - called on app shutdown
pub async fn cleanup_all_state(killer_state: &KillerState, scanner_state: &ScannerState) {
    log::info!("Cleaning up all state...");

    // Clean up killer state
    {
        let mut killer = killer_state.lock().await;
        if let Err(e) = killer.unkill_all().await {
            log::error!("Failed to unkill all devices during cleanup: {}", e);
        }
        killer.poisoned_devices.clear();
        killer.interface_name = None;
        killer.router = None;
    }

    // Clean up scanner state
    {
        let mut scanner = scanner_state.lock().await;
        scanner.is_running = false;
        scanner.discovered_devices.clear();
        scanner.current_interface = None;
        scanner.should_stop = true;
        scanner.known_devices.clear();
    }

    log::info!("State cleanup completed");
}
