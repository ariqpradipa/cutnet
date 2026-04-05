//! Bandwidth throttling implementation for CutNet
//!
//! This module provides cross-platform bandwidth limiting capabilities using:
//! - Linux: tc (traffic control) with HTB qdisc
//! - macOS: dnctl + pfctl (dummynet pipes)
//! - Windows: netsh QoS policies (experimental)

use crate::network::types::NetworkError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Represents a bandwidth limit for a specific device
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BandwidthLimit {
    pub mac: String,
    pub download_limit_kbps: Option<u32>,
    pub upload_limit_kbps: Option<u32>,
    pub enabled: bool,
}

impl BandwidthLimit {
    pub fn new(mac: impl Into<String>) -> Self {
        Self {
            mac: mac.into(),
            download_limit_kbps: None,
            upload_limit_kbps: None,
            enabled: false,
        }
    }

    pub fn with_download_limit(mut self, kbps: u32) -> Self {
        self.download_limit_kbps = Some(kbps);
        self
    }

    pub fn with_upload_limit(mut self, kbps: u32) -> Self {
        self.upload_limit_kbps = Some(kbps);
        self
    }

    pub fn enabled(mut self) -> Self {
        self.enabled = true;
        self
    }
}

/// Statistics for bandwidth usage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BandwidthStats {
    pub mac: String,
    pub current_download_kbps: u64,
    pub current_upload_kbps: u64,
    pub total_download_bytes: u64,
    pub total_upload_bytes: u64,
}

/// Errors specific to bandwidth operations
#[derive(Debug, thiserror::Error)]
pub enum BandwidthError {
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Interface error: {0}")]
    InterfaceError(String),
    #[error("Limit already set for {0}")]
    AlreadyExists(String),
    #[error("Limit not found for {0}")]
    NotFound(String),
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
    #[error("Invalid MAC address: {0}")]
    InvalidMac(String),
}

impl From<BandwidthError> for NetworkError {
    fn from(err: BandwidthError) -> Self {
        match err {
            BandwidthError::PermissionDenied(msg) => NetworkError::PermissionDenied(msg),
            BandwidthError::PlatformNotSupported(msg) => NetworkError::PlatformNotSupported(msg),
            BandwidthError::InterfaceError(msg) => NetworkError::ArpScanError(msg),
            BandwidthError::InvalidMac(msg) => NetworkError::InvalidMacAddress(msg),
            _ => NetworkError::PoisoningError(err.to_string()),
        }
    }
}

/// Platform-specific bandwidth controller
pub struct BandwidthController {
    limits: Arc<RwLock<HashMap<String, BandwidthLimit>>>,
    interface: String,
}

impl BandwidthController {
    pub fn new(interface: impl Into<String>) -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            interface: interface.into(),
        }
    }

    /// Get the interface name
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// Set a bandwidth limit for a device
    pub async fn set_limit(
        &self,
        mac: &str,
        download_kbps: Option<u32>,
        upload_kbps: Option<u32>,
    ) -> Result<(), BandwidthError> {
        // Validate MAC address
        if !Self::is_valid_mac(mac) {
            return Err(BandwidthError::InvalidMac(mac.to_string()));
        }

        let mac_normalized = mac.to_lowercase();

        // Remove existing limit first (to allow updates)
        let _ = self.remove_limit_internal(&mac_normalized).await;

        // Apply platform-specific limit
        #[cfg(target_os = "linux")]
        self.set_limit_linux(&mac_normalized, download_kbps, upload_kbps).await?;

        #[cfg(target_os = "macos")]
        self.set_limit_macos(&mac_normalized, download_kbps, upload_kbps).await?;

        #[cfg(target_os = "windows")]
        self.set_limit_windows(&mac_normalized, download_kbps, upload_kbps).await?;

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Err(BandwidthError::PlatformNotSupported(
            "Bandwidth control not supported on this platform".to_string()
        ));

        // Store the limit
        let limit = BandwidthLimit {
            mac: mac_normalized.clone(),
            download_limit_kbps: download_kbps,
            upload_limit_kbps: upload_kbps,
            enabled: true,
        };

        let mut limits = self.limits.write().await;
        limits.insert(mac_normalized, limit);

        Ok(())
    }

    /// Remove a bandwidth limit for a device
    pub async fn remove_limit(&self, mac: &str) -> Result<(), BandwidthError> {
        if !Self::is_valid_mac(mac) {
            return Err(BandwidthError::InvalidMac(mac.to_string()));
        }

        let mac_normalized = mac.to_lowercase();
        self.remove_limit_internal(&mac_normalized).await
    }

    /// Internal remove method
    async fn remove_limit_internal(&self, mac: &str) -> Result<(), BandwidthError> {
        let mac_normalized = mac.to_lowercase();

        #[cfg(target_os = "linux")]
        self.remove_limit_linux(&mac_normalized).await?;

        #[cfg(target_os = "macos")]
        self.remove_limit_macos(&mac_normalized).await?;

        #[cfg(target_os = "windows")]
        self.remove_limit_windows(&mac_normalized).await?;

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Err(BandwidthError::PlatformNotSupported(
            "Bandwidth control not supported on this platform".to_string()
        ));

        let mut limits = self.limits.write().await;
        limits.remove(&mac_normalized);

        Ok(())
    }

    /// Get all active bandwidth limits
    pub async fn get_limits(&self) -> Vec<BandwidthLimit> {
        let limits = self.limits.read().await;
        limits.values().cloned().collect()
    }

    /// Get bandwidth limit for a specific device
    pub async fn get_limit(&self, mac: &str) -> Option<BandwidthLimit> {
        let mac_normalized = mac.to_lowercase();
        let limits = self.limits.read().await;
        limits.get(&mac_normalized).cloned()
    }

    /// Get bandwidth statistics for a device
    pub async fn get_stats(&self, mac: &str) -> Result<BandwidthStats, BandwidthError> {
        if !Self::is_valid_mac(mac) {
            return Err(BandwidthError::InvalidMac(mac.to_string()));
        }

        let mac_normalized = mac.to_lowercase();

        #[cfg(target_os = "linux")]
        return self.get_stats_linux(&mac_normalized).await;

        #[cfg(target_os = "macos")]
        return self.get_stats_macos(&mac_normalized).await;

        #[cfg(target_os = "windows")]
        return self.get_stats_windows(&mac_normalized).await;

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        Err(BandwidthError::PlatformNotSupported(
            "Bandwidth stats not supported on this platform".to_string()
        ))
    }

    /// Remove all bandwidth limits
    pub async fn remove_all_limits(&self) -> Result<(), BandwidthError> {
        let macs: Vec<String> = {
            let limits = self.limits.read().await;
            limits.keys().cloned().collect()
        };

        for mac in macs {
            let _ = self.remove_limit_internal(&mac).await;
        }

        Ok(())
    }

    /// Validate MAC address format
    fn is_valid_mac(mac: &str) -> bool {
        let mac_regex = regex::Regex::new(r"^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$").unwrap();
        mac_regex.is_match(mac)
    }

    // ==================== Linux Implementation (tc) ====================

    #[cfg(target_os = "linux")]
    async fn set_limit_linux(
        &self,
        mac: &str,
        download_kbps: Option<u32>,
        upload_kbps: Option<u32>,
    ) -> Result<(), BandwidthError> {
        let iface = &self.interface;

        // Initialize HTB qdisc if not already set up
        self.init_tc_linux().await?;

        // Convert MAC to tc filter format (remove colons)
        let mac_clean = mac.replace(':', "").replace('-', "");

        // Generate unique handle for this MAC
        let handle = self.mac_to_handle(mac);

        // Set download limit (ingress traffic)
        if let Some(rate) = download_kbps {
            // Create ingress filter
            let output = Command::new("tc")
                .args([
                    "filter", "add", "dev", iface,
                    "parent", "ffff:",
                    "protocol", "all",
                    "u32", "match", "u16", "0x0800", "0xffff", "at", "-2",
                    "match", "ether", "src", mac,
                    "police", "rate", &format!("{}kbit", rate),
                    "burst", &format!("{}kbit", rate / 10),
                    "drop",
                    "flowid", &format!("1:{}", handle),
                ])
                .output()
                .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") {
                    return Err(BandwidthError::PermissionDenied(
                        "Bandwidth control requires elevated privileges (sudo)".to_string()
                    ));
                }
                log::warn!("tc filter add warning: {}", stderr);
            }
        }

        // Set upload limit (egress traffic)
        if let Some(rate) = upload_kbps {
            // Create HTB class for egress limiting
            let output = Command::new("tc")
                .args([
                    "class", "add", "dev", iface,
                    "parent", "1:",
                    "classid", &format!("1:{}", handle),
                    "htb", "rate", &format!("{}kbit", rate),
                ])
                .output()
                .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Class might already exist, that's okay
                if !stderr.contains("Class exists") {
                    log::warn!("tc class add warning: {}", stderr);
                }
            }

            // Create filter to match MAC and send to class
            let output = Command::new("tc")
                .args([
                    "filter", "add", "dev", iface,
                    "parent", "1:",
                    "protocol", "all",
                    "u32", "match", "ether", "dst", mac,
                    "flowid", &format!("1:{}", handle),
                ])
                .output()
                .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") {
                    return Err(BandwidthError::PermissionDenied(
                        "Bandwidth control requires elevated privileges (sudo)".to_string()
                    ));
                }
                log::warn!("tc filter add warning: {}", stderr);
            }
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn init_tc_linux(&self) -> Result<(), BandwidthError> {
        let iface = &self.interface;

        // Check if qdisc already exists
        let check = Command::new("tc")
            .args(["qdisc", "show", "dev", iface])
            .output()
            .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

        let output_str = String::from_utf8_lossy(&check.stdout);

        // Add HTB qdisc for egress if not exists
        if !output_str.contains("htb") {
            let output = Command::new("tc")
                .args([
                    "qdisc", "add", "dev", iface,
                    "root", "handle", "1:", "htb", "default", "12"
                ])
                .output()
                .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") {
                    return Err(BandwidthError::PermissionDenied(
                        "Bandwidth control requires elevated privileges (sudo)".to_string()
                    ));
                }
                // Might already exist
                if !stderr.contains("File exists") {
                    log::warn!("tc qdisc add warning: {}", stderr);
                }
            }
        }

        // Add ingress qdisc for download limiting if not exists
        if !output_str.contains("ingress") && !output_str.contains("ffff:") {
            let output = Command::new("tc")
                .args([
                    "qdisc", "add", "dev", iface,
                    "handle", "ffff:", "ingress"
                ])
                .output()
                .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") {
                    return Err(BandwidthError::PermissionDenied(
                        "Bandwidth control requires elevated privileges (sudo)".to_string()
                    ));
                }
                // Might already exist
                if !stderr.contains("File exists") {
                    log::warn!("tc ingress add warning: {}", stderr);
                }
            }
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn remove_limit_linux(&self, mac: &str) -> Result<(), BandwidthError> {
        let iface = &self.interface;
        let handle = self.mac_to_handle(mac);

        // Remove ingress filter (download limit)
        let _ = Command::new("tc")
            .args([
                "filter", "del", "dev", iface,
                "parent", "ffff:",
                "protocol", "all",
                "u32", "match", "ether", "src", mac,
            ])
            .output();

        // Remove egress filter (upload limit)
        let _ = Command::new("tc")
            .args([
                "filter", "del", "dev", iface,
                "parent", "1:",
                "protocol", "all",
                "u32", "match", "ether", "dst", mac,
            ])
            .output();

        // Remove HTB class
        let _ = Command::new("tc")
            .args([
                "class", "del", "dev", iface,
                "classid", &format!("1:{}", handle),
            ])
            .output();

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn get_stats_linux(&self, mac: &str) -> Result<BandwidthStats, BandwidthError> {
        let iface = &self.interface;

        // Get tc filter statistics
        let output = Command::new("tc")
            .args(["-s", "filter", "show", "dev", iface])
            .output()
            .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse stats (this is simplified - real implementation would parse tc output)
        let mut stats = BandwidthStats {
            mac: mac.to_string(),
            ..Default::default()
        };

        // Look for the MAC in the filter output
        for line in stdout.lines() {
            if line.contains(mac) || line.contains(&mac.replace(':', "")) {
                // Found matching filter, look for next lines with stats
                // Format: Sent X bytes Y pkt...
                if let Some(stats_line) = stdout.lines().skip_while(|l| !l.contains(mac)).nth(1) {
                    if stats_line.contains("Sent") {
                        // Parse bytes
                        if let Some(bytes_str) = stats_line.split("Sent").nth(1) {
                            if let Some(bytes) = bytes_str.split_whitespace().next() {
                                if let Ok(bytes_val) = bytes.parse::<u64>() {
                                    stats.total_download_bytes = bytes_val;
                                }
                            }
                        }
                    }
                }
                break;
            }
        }

        Ok(stats)
    }

    // ==================== macOS Implementation (dnctl/pfctl) ====================

    #[cfg(target_os = "macos")]
    async fn set_limit_macos(
        &self,
        mac: &str,
        download_kbps: Option<u32>,
        upload_kbps: Option<u32>,
    ) -> Result<(), BandwidthError> {
        let handle = self.mac_to_handle(mac);

        // Initialize dummynet pipes if needed
        self.init_dummynet_macos().await?;

        // Create download pipe (for incoming traffic)
        if let Some(rate) = download_kbps {
            let pipe_num = handle * 2; // Even pipes for download

            let output = Command::new("dnctl")
                .args([
                    "pipe", &pipe_num.to_string(),
                    "config", "bw", &format!("{}Kbit", rate),
                ])
                .output()
                .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("Permission denied") {
                    return Err(BandwidthError::PermissionDenied(
                        "Bandwidth control requires elevated privileges (sudo)".to_string()
                    ));
                }
                log::warn!("dnctl pipe config warning: {}", stderr);
            }

            // Add pf rule for download limiting
            self.add_pf_rule_macos(mac, pipe_num, "in").await?;
        }

        // Create upload pipe (for outgoing traffic)
        if let Some(rate) = upload_kbps {
            let pipe_num = handle * 2 + 1; // Odd pipes for upload

            let output = Command::new("dnctl")
                .args([
                    "pipe", &pipe_num.to_string(),
                    "config", "bw", &format!("{}Kbit", rate),
                ])
                .output()
                .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("Permission denied") {
                    return Err(BandwidthError::PermissionDenied(
                        "Bandwidth control requires elevated privileges (sudo)".to_string()
                    ));
                }
                log::warn!("dnctl pipe config warning: {}", stderr);
            }

            // Add pf rule for upload limiting
            self.add_pf_rule_macos(mac, pipe_num, "out").await?;
        }

        // Enable pf if not already enabled
        let _ = Command::new("pfctl")
            .args(["-e"])
            .output();

        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn init_dummynet_macos(&self) -> Result<(), BandwidthError> {
        // Check if dummynet module is loaded
        let check = Command::new("kextstat")
            .arg("-l")
            .output()
            .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

        let output_str = String::from_utf8_lossy(&check.stdout);
        if !output_str.contains("dummynet") {
            // Try to load dummynet
            let _ = Command::new("kextload")
                .arg("/System/Library/Extensions/dummynet.kext")
                .output();
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn add_pf_rule_macos(
        &self,
        mac: &str,
        pipe_num: u32,
        direction: &str,
    ) -> Result<(), BandwidthError> {
        // Create a temporary pf.conf file with the rule
        let rule = if direction == "in" {
            format!("pass in on {} from any to any mac {} dnpipe {}", 
                self.interface, mac, pipe_num)
        } else {
            format!("pass out on {} from any to any mac {} dnpipe {}", 
                self.interface, mac, pipe_num)
        };

        // Append rule to pf anchors
        let anchor_name = format!("cutnet_{}", mac.replace(':', ""));

        let output = Command::new("pfctl")
            .args([
                "-a", &anchor_name,
                "-f", "-",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

        if let Some(mut stdin) = output.stdin {
            use std::io::Write;
            let _ = stdin.write_all(rule.as_bytes());
        }

        let result = output.wait_with_output()
            .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            if stderr.contains("Permission denied") {
                return Err(BandwidthError::PermissionDenied(
                    "Bandwidth control requires elevated privileges (sudo)".to_string()
                ));
            }
            log::warn!("pfctl rule add warning: {}", stderr);
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn remove_limit_macos(&self, mac: &str) -> Result<(), BandwidthError> {
        let handle = self.mac_to_handle(mac);

        // Remove pipes
        let _ = Command::new("dnctl")
            .args(["pipe", &format!("{}", handle * 2), "delete"])
            .output();

        let _ = Command::new("dnctl")
            .args(["pipe", &format!("{}", handle * 2 + 1), "delete"])
            .output();

        // Remove pf anchor
        let anchor_name = format!("cutnet_{}", mac.replace(':', ""));

        let _ = Command::new("pfctl")
            .args(["-a", &anchor_name, "-F", "all"])
            .output();

        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn get_stats_macos(&self, mac: &str) -> Result<BandwidthStats, BandwidthError> {
        let handle = self.mac_to_handle(mac);
        let mut stats = BandwidthStats {
            mac: mac.to_string(),
            ..Default::default()
        };

        // Get pipe statistics
        let output = Command::new("dnctl")
            .args(["pipe", "show"])
            .output()
            .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse download pipe stats
        for (i, line) in stdout.lines().enumerate() {
            if line.contains(&format!("0000{}:", handle * 2)) {
                if let Some(next_line) = stdout.lines().nth(i + 1) {
                    if let Some(bytes_str) = next_line.split("bytes:").nth(1) {
                        if let Ok(bytes) = bytes_str.trim().parse::<u64>() {
                            stats.total_download_bytes = bytes;
                        }
                    }
                }
            }
            if line.contains(&format!("0000{}:", handle * 2 + 1)) {
                if let Some(next_line) = stdout.lines().nth(i + 1) {
                    if let Some(bytes_str) = next_line.split("bytes:").nth(1) {
                        if let Ok(bytes) = bytes_str.trim().parse::<u64>() {
                            stats.total_upload_bytes = bytes;
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    // ==================== Windows Implementation (netsh QoS) ====================

    #[cfg(target_os = "windows")]
    async fn set_limit_windows(
        &self,
        _mac: &str,
        download_kbps: Option<u32>,
        upload_kbps: Option<u32>,
    ) -> Result<(), BandwidthError> {
        // Windows QoS with netsh is experimental and limited
        // It works better with IP-based policies rather than MAC-based

        log::warn!("Windows bandwidth limiting is experimental and uses netsh QoS policies");
        log::warn!("Consider using Windows Filtering Platform (WFP) for production use");

        // Create QoS policy
        let rate = download_kbps.or(upload_kbps).unwrap_or(1000);

        let output = Command::new("netsh")
            .args([
                "advfirewall", "qos", "add", "rule",
                "name=CutNetBandwidthLimit",
                &format!("rate={}", rate),
                "profile=all",
            ])
            .output()
            .map_err(|e| BandwidthError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Access is denied") {
                return Err(BandwidthError::PermissionDenied(
                    "Bandwidth control requires administrator privileges".to_string()
                ));
            }
            return Err(BandwidthError::CommandFailed(stderr.to_string()));
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn remove_limit_windows(&self, _mac: &str) -> Result<(), BandwidthError> {
        let _ = Command::new("netsh")
            .args([
                "advfirewall", "qos", "delete", "rule",
                "name=CutNetBandwidthLimit",
            ])
            .output();

        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn get_stats_windows(&self, _mac: &str) -> Result<BandwidthStats, BandwidthError> {
        // Windows netsh doesn't provide per-MAC bandwidth stats easily
        // Would need to use performance counters or WFP

        Ok(BandwidthStats {
            mac: _mac.to_string(),
            ..Default::default()
        })
    }

    // ==================== Helper Functions ====================

    /// Convert MAC address to a numeric handle for tc/dummynet
    fn mac_to_handle(&self, mac: &str) -> u32 {
        // Use last 4 hex digits of MAC to create a handle
        let clean = mac.replace(':', "").replace('-', "");
        let last4 = &clean[clean.len().saturating_sub(4)..];
        u32::from_str_radix(last4, 16).unwrap_or(1)
    }
}

/// Global bandwidth controller instance
use once_cell::sync::Lazy;
use std::sync::Mutex;

static BANDWIDTH_CONTROLLER: Lazy<Mutex<Option<Arc<BandwidthController>>>> = 
    Lazy::new(|| Mutex::new(None));

/// Initialize the global bandwidth controller
pub fn init_bandwidth_controller(interface: impl Into<String>) -> Arc<BandwidthController> {
    let controller = Arc::new(BandwidthController::new(interface));
    let mut global = BANDWIDTH_CONTROLLER.lock().unwrap();
    *global = Some(controller.clone());
    controller
}

/// Get the global bandwidth controller
pub fn get_bandwidth_controller() -> Option<Arc<BandwidthController>> {
    BANDWIDTH_CONTROLLER.lock().unwrap().clone()
}

/// Shutdown the bandwidth controller and remove all limits
pub async fn shutdown_bandwidth_controller() -> Result<(), BandwidthError> {
    if let Some(controller) = get_bandwidth_controller() {
        controller.remove_all_limits().await?;
    }
    Ok(())
}
