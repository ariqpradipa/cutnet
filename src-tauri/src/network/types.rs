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
    pub ip: String,
    pub mac: String,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub is_router: bool,
    pub is_me: bool,
    pub custom_name: Option<String>,
}

impl Device {
    pub fn new(ip: impl Into<String>, mac: impl Into<String>) -> Self {
        Self {
            ip: ip.into(),
            mac: mac.into(),
            hostname: None,
            vendor: None,
            is_router: false,
            is_me: false,
            custom_name: None,
        }
    }

    pub fn as_router(mut self) -> Self {
        self.is_router = true;
        self
    }

    #[allow(dead_code)]
    pub fn as_me(mut self) -> Self {
        self.is_me = true;
        self
    }

    #[allow(dead_code)]
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
        self
    }

    pub fn with_vendor(mut self, vendor: impl Into<String>) -> Self {
        self.vendor = Some(vendor.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_custom_name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }
}

/// Represents a network interface (NIC) on the local machine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    pub mac: String,
    pub broadcast_addr: String,
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
    #[allow(dead_code)]
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
#[allow(dead_code)]
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

    #[error("Invalid MAC address: {0} - {1}")]
    MacValidationError(String, MacValidationError),

    #[error("Invalid IP address format: {0}")]
    InvalidIpAddress(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Bandwidth control error: {0}")]
    BandwidthError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Packet forwarding error: {0}")]
    ForwardingError(String),

    #[error("Connection tracking error: {0}")]
    ConnectionTrackError(String),

    #[error("IP forwarding not enabled on system")]
    IpForwardingDisabled,
}

pub type Result<T> = std::result::Result<T, NetworkError>;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MacValidationError {
    BroadcastAddress,
    MulticastAddress,
    AllZeros,
}

impl std::fmt::Display for MacValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MacValidationError::BroadcastAddress => write!(f, "broadcast"),
            MacValidationError::MulticastAddress => write!(f, "multicast"),
            MacValidationError::AllZeros => write!(f, "all-zeros"),
        }
    }
}

// ===== Schedule Types =====

/// Action to perform for a scheduled event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action_type")]
pub enum ScheduleAction {
    /// Kill the target device
    Kill,
    /// Restore the target device
    Restore,
    /// Kill for a duration then auto-restore
    KillAndRestore { duration_minutes: u32 },
}

/// Type of schedule timing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "schedule_type")]
pub enum ScheduleType {
    /// One-time execution at a specific timestamp
    OneTime { execute_at: u64 },
    /// Daily execution at a specific time
    Daily { time: TimeOfDay },
    /// Weekly execution on specific days
    Weekly {
        days: Vec<DayOfWeek>,
        time: TimeOfDay,
    },
}

/// Time of day for scheduled execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
}

/// Day of week for weekly schedules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DayOfWeek {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl DayOfWeek {
    pub fn from_chrono(day: chrono::Weekday) -> Self {
        match day {
            chrono::Weekday::Mon => DayOfWeek::Monday,
            chrono::Weekday::Tue => DayOfWeek::Tuesday,
            chrono::Weekday::Wed => DayOfWeek::Wednesday,
            chrono::Weekday::Thu => DayOfWeek::Thursday,
            chrono::Weekday::Fri => DayOfWeek::Friday,
            chrono::Weekday::Sat => DayOfWeek::Saturday,
            chrono::Weekday::Sun => DayOfWeek::Sunday,
        }
    }
}

/// A scheduled kill/restore operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSchedule {
    pub id: String,
    pub device_mac: String,
    pub device_ip: String,
    pub action: ScheduleAction,
    pub schedule_type: ScheduleType,
    pub enabled: bool,
    pub created_at: u64,
    pub timezone_offset: i32,
}

// ===== Forwarding Types =====

/// Action for a forwarding rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForwardAction {
    Allow,
    Block,
    Log,
    Modify,
}

/// Protocol for forwarding rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    TCP,
    UDP,
    ICMP,
    All,
}

/// Direction of a packet in forwarding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PacketDirection {
    VictimToRouter,
    RouterToVictim,
}

/// TCP state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TcpState {
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
    Closed,
}

/// A rule for packet forwarding filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardingRule {
    pub id: String,
    pub protocol: Protocol,
    pub port: Option<u16>,
    pub action: ForwardAction,
    pub description: Option<String>,
}

/// Configuration for packet forwarding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardingConfig {
    pub enabled: bool,
    pub victim_mac: String,
    pub router_mac: String,
    pub interface_name: String,
    #[serde(skip)]
    pub forward_stats: ForwardStats,
}

/// Statistics for packet forwarding
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForwardStats {
    pub packets_forwarded: u64,
    pub bytes_forwarded: u64,
    pub packets_dropped: u64,
    pub bytes_dropped: u64,
    pub packets_modified: u64,
    pub active_connections: u64,
}

// ===== Connection Tracking Types =====

/// Information about a tracked connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: Protocol,
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub packets_received: u64,
    pub bytes_received: u64,
    pub state: TcpState,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl ConnectionInfo {
    pub fn new(
        src_ip: impl Into<String>,
        dst_ip: impl Into<String>,
        src_port: u16,
        dst_port: u16,
        protocol: Protocol,
    ) -> Self {
        Self {
            src_ip: src_ip.into(),
            dst_ip: dst_ip.into(),
            src_port,
            dst_port,
            protocol,
            packets_sent: 0,
            bytes_sent: 0,
            packets_received: 0,
            bytes_received: 0,
            state: TcpState::Established,
            last_activity: chrono::Utc::now(),
        }
    }
}

// ===== Kill Target Types (MAC-based Persistent Tracking) =====

/// Represents a kill target - tracks by MAC with mutable IP
/// 
/// This structure is MAC-centric to prevent DHCP/IP renewal bypass.
/// The MAC address is hardware-based and constant, while IP can change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillTarget {
    /// MAC address (immutable identifier)
    pub mac: String,
    /// Current IP address (can change on DHCP renewal)
    pub ip: String,
    /// Router/gateway device
    pub router: Device,
    /// Interface name for poisoning
    pub interface_name: String,
    /// When this device was first killed (UNIX timestamp)
    pub killed_at: u64,
    /// Whether poisoning is currently active
    pub is_active: bool,
}

impl KillTarget {
    pub fn new(
        mac: impl Into<String>,
        ip: impl Into<String>,
        router: Device,
        interface_name: impl Into<String>,
    ) -> Self {
        Self {
            mac: mac.into(),
            ip: ip.into(),
            router,
            interface_name: interface_name.into(),
            killed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            is_active: true,
        }
    }
}

/// State of a kill target from persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentKillTarget {
    /// MAC address (persistent identifier)
    pub mac: String,
    /// First observed IP when killed
    pub first_seen_ip: String,
    /// When this device was first killed (UNIX timestamp)
    pub killed_at: u64,
    /// Whether to auto-kill when this MAC is detected
    pub auto_kill: bool,
}

impl PersistentKillTarget {
    pub fn new(mac: impl Into<String>, ip: impl Into<String>) -> Self {
        Self {
            mac: mac.into(),
            first_seen_ip: ip.into(),
            killed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            auto_kill: true,
        }
    }
}
