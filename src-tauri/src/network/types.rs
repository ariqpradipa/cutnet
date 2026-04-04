//! Core data structures for network operations
//!
//! This module defines the main data types used throughout CutNet for
//! representing network devices, interfaces, and operational states.

use serde::{Deserialize, Serialize};

/// Represents a network device discovered on the local network
///
/// A Device captures information about a network participant including
/// its IP and MAC addresses, optional hostname and vendor identification,
/// and special flags indicating if it's the router or the local machine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Device {
    /// IP address of the device (e.g., "192.168.1.100")
    pub ip: String,

    /// MAC address of the device (e.g., "aa:bb:cc:dd:ee:ff")
    pub mac: String,

    /// Optional hostname from reverse DNS lookup
    pub hostname: Option<String>,

    /// Optional vendor name based on MAC OUI lookup
    pub vendor: Option<String>,

    /// Flag indicating if this device is the network gateway/router
    pub is_router: bool,

    /// Flag indicating if this device represents the local machine
    pub is_me: bool,
}

impl Device {
    /// Create a new Device with the given IP and MAC
    pub fn new(ip: impl Into<String>, mac: impl Into<String>) -> Self {
        Self {
            ip: ip.into(),
            mac: mac.into(),
            hostname: None,
            vendor: None,
            is_router: false,
            is_me: false,
        }
    }

    /// Mark this device as the router
    pub fn as_router(mut self) -> Self {
        self.is_router = true;
        self
    }

    /// Mark this device as the local machine
    pub fn as_me(mut self) -> Self {
        self.is_me = true;
        self
    }

    /// Set the hostname
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
        self
    }

    /// Set the vendor
    pub fn with_vendor(mut self, vendor: impl Into<String>) -> Self {
        self.vendor = Some(vendor.into());
        self
    }
}

/// Represents a network interface (NIC) on the local machine
///
/// This struct contains information about a network interface including
/// its name, IP configuration, MAC address, and network range.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkInterface {
    /// Interface name (e.g., "eth0", "en0", "Wi-Fi")
    pub name: String,

    /// IP address assigned to this interface
    pub ip: String,

    /// MAC address of this interface
    pub mac: String,

    /// Broadcast address for this network (e.g., "192.168.1.255")
    pub broadcast_addr: String,

    /// Netmask for this interface (e.g., "255.255.255.0")
    pub netmask: String,
}

impl NetworkInterface {
    /// Create a new NetworkInterface
    pub fn new(
        name: impl Into<String>,
        ip: impl Into<String>,
        mac: impl Into<String>,
        broadcast_addr: impl Into<String>,
        netmask: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            ip: ip.into(),
            mac: mac.into(),
            broadcast_addr: broadcast_addr.into(),
            netmask: netmask.into(),
        }
    }

    /// Calculate the network prefix from the IP and netmask
    /// Returns something like "192.168.1"
    pub fn network_prefix(&self) -> Option<String> {
        let ip_parts: Vec<&str> = self.ip.split('.').collect();
        let mask_parts: Vec<&str> = self.netmask.split('.').collect();

        if ip_parts.len() != 4 || mask_parts.len() != 4 {
            return None;
        }

        // Calculate how many octets are fully covered by the netmask
        let mut prefix_octets = Vec::new();
        for (ip_octet, mask_octet) in ip_parts.iter().zip(mask_parts.iter()) {
            let ip_val: u8 = ip_octet.parse().ok()?;
            let mask_val: u8 = mask_octet.parse().ok()?;

            if mask_val == 255 {
                prefix_octets.push(ip_val);
            } else if mask_val == 0 {
                break;
            } else {
                // Partial octet - include the masked bits
                prefix_octets.push(ip_val & mask_val);
                break;
            }
        }

        Some(
            prefix_octets
                .iter()
                .map(|o| o.to_string())
                .collect::<Vec<_>>()
                .join("."),
        )
    }
}

/// Represents the state of an ARP poisoning operation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PoisoningState {
    /// Poisoning is not active
    Idle,
    /// Poisoning is currently running
    Active,
    /// Poisoning is stopping (sending restore packets)
    Stopping,
}

/// Configuration for ARP poisoning operations
#[derive(Debug, Clone)]
pub struct PoisoningConfig {
    /// Interval between ARP packets in milliseconds (default: 2000)
    pub interval_ms: u64,

    /// Number of restore packets to send when stopping (default: 3)
    pub restore_count: u8,

    /// Interval between restore packets in milliseconds (default: 100)
    pub restore_interval_ms: u64,
}

impl Default for PoisoningConfig {
    fn default() -> Self {
        Self {
            interval_ms: 2000,        // 2 seconds matching ArpCut behavior
            restore_count: 3,         // Send 3 restore packets
            restore_interval_ms: 100, // 100ms between restores
        }
    }
}

/// Error types for network operations
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Interface not found: {0}")]
    InterfaceNotFound(String),

    #[error("Failed to get MAC address: {0}")]
    MacAddressError(String),

    #[error("Failed to set MAC address: {0}")]
    MacSetError(String),

    #[error("ARP scan failed: {0}")]
    ArpScanError(String),

    #[error("Ping scan failed: {0}")]
    PingScanError(String),

    #[error("Poisoning operation failed: {0}")]
    PoisoningError(String),

    #[error("Raw socket creation failed: {0}")]
    RawSocketError(String),

    #[error("Packet send failed: {0}")]
    PacketSendError(String),

    #[error("Invalid MAC address format: {0}")]
    InvalidMacAddress(String),

    #[error("Invalid IP address format: {0}")]
    InvalidIpAddress(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, NetworkError>;
