# CutNet Product Requirements Documents (PRDs)

**Project:** CutNet - Network Administration Tool  
**Version:** 0.2.0 (Planned)  
**Date:** April 5, 2026

---

## PRIORITY 1: CRITICAL BUG FIXES

### PRD-1.1: Fix ARP Scan Race Condition (BUG-003)

**Problem:**  
The current ARP scan implementation in `scanner.rs:30-69` creates a channel and spawns receiver/sender tasks simultaneously. On high-latency systems, ARP replies may arrive before the receiver task is ready, causing missed device detections.

**Current Code Flow:**
```rust
let (mut tx, mut rx) = create_arp_channel(&interface)?;
let recv_task = tokio::spawn(async move { receive_arp_replies(&mut rx, ...).await; });
let send_task = tokio::spawn(async move { send_arp_requests(&mut tx, ...).await; });
// Race: send_task may send before recv_task is listening
```

**Requirements:**
- [ ] Ensure receiver is ready and listening BEFORE first ARP request is sent
- [ ] Add synchronization barrier between task spawns
- [ ] Add ready signal from receiver to sender
- [ ] Document the synchronization mechanism in code comments

**Technical Solution:**
```rust
use tokio::sync::oneshot;

let (mut tx, mut rx) = create_arp_channel(&interface)?;
let (ready_tx, ready_rx) = oneshot::channel();

let recv_task = tokio::spawn(async move {
    // Signal ready before starting to listen
    let _ = ready_tx.send(());
    receive_arp_replies(&mut rx, &recv_discovered).await;
});

// Wait for receiver to be ready
let _ = ready_rx.await;

let send_task = tokio::spawn(async move {
    tokio::time::sleep(Duration::from_millis(10)).await; // Small buffer
    send_arp_requests(&mut tx, ...).await;
});
```

**Success Criteria:**
- ARP scan discovers 100% of devices on test network (verified with 3+ runs)
- No devices missed due to timing issues
- Code review approves synchronization mechanism

**Files to Modify:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/scanner.rs` (lines 30-76)

---

### PRD-1.2: Fix UI Optimistic State Updates (BUG-006)

**Problem:**  
`DeviceTable.tsx:256-291` updates the kill state in the UI BEFORE awaiting the backend confirmation, causing false success states when operations fail.

**Current Code:**
```typescript
setKillState(device.mac, { mac: device.mac, is_killed: true, kill_type: "arp_poison" });
await killDevice(device); // If this fails, UI still shows killed
```

**Requirements:**
- [ ] Update UI state ONLY after successful backend confirmation
- [ ] Show loading state during operation
- [ ] Rollback UI state on failure with error toast
- [ ] Add retry mechanism for failed operations

**Technical Solution:**
```typescript
const handleKillToggle = async (device: Device) => {
    const isCurrentlyKilled = killStates.get(device.mac)?.is_killed;
    
    // Set loading state
    setKillState(device.mac, { ...currentState, is_pending: true });
    
    try {
        if (isCurrentlyKilled) {
            await unkillDevice(device);
            setKillState(device.mac, { mac: device.mac, is_killed: false, kill_type: "none" });
            showToast('success', `Restored ${getDeviceName(device)}`);
        } else {
            await killDevice(device);
            setKillState(device.mac, { mac: device.mac, is_killed: true, kill_type: "arp_poison" });
            showToast('success', `Blocked ${getDeviceName(device)}`);
        }
    } catch (error) {
        // Rollback to previous state
        setKillState(device.mac, { ...currentState, is_pending: false });
        showToast('error', `Failed: ${errorMessage(error)}`);
    }
};
```

**Success Criteria:**
- UI never shows killed state when backend operation fails
- Loading indicator visible during operations
- Error toasts show on failure
- Manual QA: Kill/unkill 10 devices, verify UI matches actual state

**Files to Modify:**
- `/Users/encore/Documents/personal/repositories/cutnet/src/components/DeviceTable.tsx` (lines 256-291)
- `/Users/encore/Documents/personal/repositories/cutnet/src/hooks/useToast.ts` (add error toast types)

---

### PRD-1.3: Add MAC Address Validation (VULN-001)

**Problem:**  
`poisoner.rs:145-174` accepts any MAC address string without validating content beyond format, risking accidental broadcast/multicast poisoning.

**Requirements:**
- [ ] Reject broadcast MAC addresses (FF:FF:FF:FF:FF:FF)
- [ ] Reject multicast MAC addresses (first octet LSB = 1)
- [ ] Reject all-zero MAC addresses (00:00:00:00:00:00)
- [ ] Reject loopback/VM-specific MAC patterns that could cause issues
- [ ] Return structured error with specific rejection reason

**Technical Solution:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum MacValidationError {
    #[error("Broadcast MAC address not allowed")]
    Broadcast,
    #[error("Multicast MAC address not allowed")]
    Multicast,
    #[error("All-zero MAC address not allowed")]
    AllZeros,
    #[error("Invalid MAC address format: {0}")]
    InvalidFormat(String),
}

pub fn validate_unicast_mac(mac: &str) -> Result<[u8; 6], MacValidationError> {
    let bytes = parse_mac_bytes(mac)?;
    
    // Check for all zeros
    if bytes.iter().all(|&b| b == 0) {
        return Err(MacValidationError::AllZeros);
    }
    
    // Check for broadcast
    if bytes.iter().all(|&b| b == 0xFF) {
        return Err(MacValidationError::Broadcast);
    }
    
    // Check for multicast (LSB of first octet)
    if bytes[0] & 0x01 != 0 {
        return Err(MacValidationError::Multicast);
    }
    
    Ok(bytes)
}
```

**Success Criteria:**
- All invalid MAC types rejected with appropriate error messages
- Unit tests cover all validation cases
- Frontend shows clear validation errors to users

**Files to Modify:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/utils.rs` (add validation)
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/poisoner.rs` (use validation)
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/types.rs` (add error types)

---

### PRD-1.4: Implement Structured Error Types for IPC (BUG-001)

**Problem:**  
`commands.rs:165-183` converts all errors to generic strings, losing type information needed for frontend error handling.

**Requirements:**
- [ ] Define error code enum for all error types
- [ ] Add retryable flag to errors
- [ ] Add user-facing message separate from technical details
- [ ] Pass structured errors through Tauri IPC

**Technical Solution:**
```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum ErrorCode {
    PermissionDenied { resource: String },
    InterfaceNotFound { name: String },
    DeviceNotFound { ip: String },
    AlreadyPoisoned { ip: String },
    NotPoisoned { ip: String },
    NetworkError { message: String },
    ValidationError { field: String, message: String },
    InternalError { message: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub user_message: String,
    pub technical_details: Option<String>,
    pub retryable: bool,
    pub suggested_action: Option<String>,
}

impl From<NetworkError> for ApiError {
    fn from(err: NetworkError) -> Self {
        match err {
            NetworkError::PermissionDenied(resource) => ApiError {
                code: ErrorCode::PermissionDenied { resource },
                user_message: "Administrator privileges required".to_string(),
                technical_details: Some(format!("Access denied to {}", resource)),
                retryable: false,
                suggested_action: Some("Run as administrator/root".to_string()),
            },
            // ... other mappings
        }
    }
}
```

**Frontend Integration:**
```typescript
// ipc.ts
export class CutNetError extends Error {
    constructor(
        message: string,
        public code: ErrorCode,
        public retryable: boolean,
        public suggestedAction?: string
    ) {
        super(message);
    }
}

export async function killDevice(device: Device): Promise<void> {
    try {
        await invoke("kill_device", { ip: device.ip, mac: device.mac });
    } catch (error: any) {
        if (error.code) {
            throw new CutNetError(
                error.user_message,
                error.code,
                error.retryable,
                error.suggested_action
            );
        }
        throw error;
    }
}
```

**Success Criteria:**
- All NetworkError variants mapped to ApiError
- Frontend receives structured errors with codes
- Error toasts show user-friendly messages
- Unit tests verify error mappings

**Files to Modify:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/ipc/error.rs` (NEW FILE)
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/ipc/commands.rs` (return ApiError)
- `/Users/encore/Documents/personal/repositories/cutnet/src/utils/ipc.ts` (parse structured errors)
- `/Users/encore/Documents/personal/repositories/cutnet/src/components/ToastContainer.tsx` (show error actions)

---

### PRD-1.5: Remove/Fix Fake Update Check (BUG-008)

**Problem:**  
`SettingsPanel.tsx:388-398` simulates update check with hardcoded 1.5s delay, always returning "up to date".

**Requirements:**
- [ ] Implement real update check against GitHub releases
- [ ] Show current version from package.json
- [ ] Compare with latest release tag
- [ ] Show download link if update available
- [ ] Handle network errors gracefully

**Technical Solution:**
```typescript
// src/lib/updateChecker.ts
const GITHUB_RELEASES_URL = 'https://api.github.com/repos/cutnet/cutnet/releases/latest';
const CURRENT_VERSION = '0.1.0'; // Read from package.json at build time

interface ReleaseInfo {
    tag_name: string;
    name: string;
    body: string;
    html_url: string;
    published_at: string;
}

export async function checkForUpdates(): Promise<{
    available: boolean;
    currentVersion: string;
    latestVersion?: string;
    releaseUrl?: string;
    releaseNotes?: string;
}> {
    try {
        const response = await fetch(GITHUB_RELEASES_URL, {
            headers: { 'Accept': 'application/vnd.github.v3+json' }
        });
        
        if (!response.ok) {
            throw new Error('Failed to fetch releases');
        }
        
        const release: ReleaseInfo = await response.json();
        const latestVersion = release.tag_name.replace('v', '');
        
        return {
            available: compareVersions(latestVersion, CURRENT_VERSION) > 0,
            currentVersion: CURRENT_VERSION,
            latestVersion,
            releaseUrl: release.html_url,
            releaseNotes: release.body
        };
    } catch (error) {
        console.error('Update check failed:', error);
        return { available: false, currentVersion: CURRENT_VERSION };
    }
}
```

**Success Criteria:**
- Real GitHub API call made
- Current version displayed correctly
- Update notification shown when new version exists
- Graceful error handling for offline/network issues

**Files to Modify:**
- `/Users/encore/Documents/personal/repositories/cutnet/src/lib/updateChecker.ts` (NEW FILE)
- `/Users/encore/Documents/personal/repositories/cutnet/src/components/SettingsPanel.tsx` (use real checker)
- `/Users/encore/Documents/personal/repositories/cutnet/package.json` (ensure version is set)

---

## PRIORITY 2: MISSING CORE FEATURES

### PRD-2.1: Bandwidth Throttling

**Problem:**  
CutNet has no bandwidth throttling capability, a core NetCut feature. Users cannot limit upload/download speeds for specific devices.

**Requirements:**
- [ ] Set per-device upload speed limits (0 = unlimited, 1-10000 KB/s)
- [ ] Set per-device download speed limits
- [ ] Apply limits in real-time without disrupting connection
- [ ] Show current bandwidth usage per device
- [ ] Support removing limits (restore to unlimited)
- [ ] Persist limits across app restarts
- [ ] Work on Windows, macOS, and Linux

**Technical Approach:**

**Linux (using `tc` - traffic control):**
```rust
// src-tauri/src/network/bandwidth.rs
#[cfg(target_os = "linux")]
pub fn set_bandwidth_limit(mac: &str, download_kbps: u32, upload_kbps: u32) -> Result<(), BandwidthError> {
    // Use tc with ingress/egress queuing disciplines
    // Create HTB (Hierarchical Token Bucket) qdisc
    // Add filter for target MAC
    // Set rate limits
    
    Command::new("tc")
        .args([
            "qdisc", "add", "dev", &interface,
            "root", "handle", "1:", "htb", "default", "30"
        ])
        .status()?;
    
    // Add class with rate limit
    Command::new("tc")
        .args([
            "class", "add", "dev", &interface,
            "parent", "1:", "classid", "1:1",
            "htb", "rate", &format!("{}kbit", download_kbps)
        ])
        .status()?;
}
```

**macOS (using `pfctl` + `dnctl` - packet filter + dummynet):**
```rust
#[cfg(target_os = "macos")]
pub fn set_bandwidth_limit(mac: &str, download_kbps: u32, upload_kbps: u32) -> Result<(), BandwidthError> {
    // Use dnctl to create pipe with bandwidth limit
    // Use pfctl to redirect traffic through pipe
    
    Command::new("dnctl")
        .args(["pipe", "1", "config", &format!("bw {}Kbit", download_kbps)])
        .status()?;
    
    // Add pf rule to match MAC and send through pipe
    // (requires elevated privileges)
}
```

**Windows (using Windows Filtering Platform - WFP):**
```rust
#[cfg(target_os = "windows")]
pub fn set_bandwidth_limit(mac: &str, download_kbps: u32, upload_kbps: u32) -> Result<(), BandwidthError> {
    // Use WFP API to create callout driver
    // This requires kernel-mode driver - complex
    // Alternative: Use netsh with QoS policies
    
    Command::new("netsh")
        .args([
            "advfirewall", "qos", "add", "rule",
            "name=CutNetLimit",
            &format!("{}={}", mac, download_kbps)
        ])
        .status()?;
}
```

**Data Structures:**
```rust
// types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthLimit {
    pub mac: String,
    pub download_limit_kbps: Option<u32>, // None = unlimited
    pub upload_limit_kbps: Option<u32>,
    pub current_download_kbps: u32, // Real-time monitoring
    pub current_upload_kbps: u32,
    pub enabled: bool,
}

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
}
```

**Frontend UI:**
```typescript
// DeviceTable.tsx - Add bandwidth column
interface BandwidthControlProps {
    device: Device;
    limit: BandwidthLimit | null;
    onSetLimit: (mac: string, download: number, upload: number) => Promise<void>;
    onRemoveLimit: (mac: string) => Promise<void>;
}

// Settings Panel - Add Bandwidth tab
interface BandwidthSettings {
    defaultDownloadLimit: number;
    defaultUploadLimit: number;
    enableByDefault: boolean;
}
```

**Success Criteria:**
- Set 1 MB/s download limit on test device, verify speed capped
- Set 500 KB/s upload limit, verify speed capped
- Remove limits, verify speed returns to normal
- Limits persist after app restart
- Works on at least 2 platforms (Linux, macOS)

**Files to Create:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/bandwidth.rs`
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/ipc/bandwidth_commands.rs`
- `/Users/encore/Documents/personal/repositories/cutnet/src/components/BandwidthControl.tsx`

**Files to Modify:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/mod.rs`
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/ipc/commands.rs`
- `/Users/encore/Documents/personal/repositories/cutnet/src/components/DeviceTable.tsx`
- `/Users/encore/Documents/personal/repositories/cutnet/src/stores/deviceStore.ts`

---

### PRD-2.2: Kill Timers/Scheduling

**Problem:**  
No scheduling capability exists. Users cannot set automatic kill/restore times for devices (e.g., block gaming console during homework hours).

**Requirements:**
- [ ] Create one-time scheduled actions (kill/restore at specific time)
- [ ] Create recurring schedules (e.g., Mon-Fri 9PM-7AM)
- [ ] Support multiple schedules per device
- [ ] Show active schedules in UI
- [ ] Enable/disable schedules without deleting
- [ ] Persist schedules across restarts
- [ ] Show next scheduled action countdown
- [ ] Handle timezone correctly

**Data Structures:**
```rust
// types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSchedule {
    pub id: String,
    pub device_mac: String,
    pub device_ip: String,
    pub action: ScheduleAction,
    pub schedule_type: ScheduleType,
    pub enabled: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleAction {
    Kill,
    Restore,
    KillAndRestore { duration_minutes: u32 }, // Kill for X minutes then auto-restore
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleType {
    OneTime { execute_at: u64 }, // Unix timestamp
    Daily { time: TimeOfDay }, // Every day at specific time
    Weekly { days: Vec<DayOfWeek>, time: TimeOfDay },
    Cron { expression: String }, // Full cron syntax
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeOfDay {
    pub hour: u8, // 0-23
    pub minute: u8, // 0-59
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DayOfWeek {
    Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday,
}
```

**Scheduler Service:**
```rust
// scheduler.rs
pub struct SchedulerService {
    schedules: RwLock<HashMap<String, KillSchedule>>,
    scheduler_handle: Mutex<Option<JoinHandle<()>>>,
}

impl SchedulerService {
    pub async fn start_scheduler(app: AppHandle) {
        let handle = tokio::spawn(async move {
            scheduler_loop(app).await;
        });
    }
}

async fn scheduler_loop(app: AppHandle) {
    let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute
    
    loop {
        interval.tick().await;
        
        let now = current_timestamp();
        let schedules = SCHEDULES.read().await;
        
        for schedule in schedules.values() {
            if !schedule.enabled {
                continue;
            }
            
            if should_execute(&schedule.schedule_type, now) {
                execute_schedule(&app, schedule).await;
                
                if let ScheduleType::OneTime { .. } = schedule.schedule_type {
                    drop(schedules);
                    delete_schedule(&schedule.id).await;
                }
            }
        }
    }
}
```

**Frontend UI:**
```typescript
// SchedulePanel.tsx
interface SchedulePanelProps {
    device: Device;
    schedules: KillSchedule[];
    onCreateSchedule: (schedule: Omit<KillSchedule, 'id' | 'created_at'>) => Promise<void>;
    onDeleteSchedule: (id: string) => Promise<void>;
    onToggleSchedule: (id: string, enabled: boolean) => Promise<void>;
}

// Schedule types in UI:
type ScheduleUI = 
    | { type: 'onetime', dateTime: string }
    | { type: 'daily', time: string }
    | { type: 'weekly', days: number[], time: string };
```

**IPC Commands:**
```rust
// commands.rs
#[tauri::command]
pub async fn create_schedule(
    device_mac: String,
    device_ip: String,
    action: ScheduleAction,
    schedule_type: ScheduleType,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    let schedule = KillSchedule {
        id: id.clone(),
        device_mac,
        device_ip,
        action,
        schedule_type,
        enabled: true,
        created_at: current_timestamp(),
    };
    
    save_schedule(&schedule).await?;
    Ok(id)
}

#[tauri::command]
pub async fn get_schedules(device_mac: String) -> Result<Vec<KillSchedule>, String> {
    Ok(get_all_schedules().await.into_iter()
        .filter(|s| s.device_mac == device_mac)
        .collect())
}
```

**Success Criteria:**
- Create one-time schedule, verify executes at correct time
- Create daily schedule, verify executes every day at specified time
- Create weekly schedule (Mon-Fri 9PM), verify correct days only
- Disable schedule, verify no execution
- Schedules survive app restart
- Timezone handling correct (schedule at 9PM local time)

**Files to Create:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/scheduler.rs`
- `/Users/encore/Documents/personal/repositories/cutnet/src/components/SchedulePanel.tsx`
- `/Users/encore/Documents/personal/repositories/cutnet/src/lib/scheduleUtils.ts`

**Files to Modify:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/ipc/commands.rs`
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/lib.rs` (start scheduler on app init)
- `/Users/encore/Documents/personal/repositories/cutnet/src/components/DeviceTable.tsx` (add schedule button)
- `/Users/encore/Documents/personal/repositories/cutnet/src/App.tsx` (add schedules panel tab)

---

### PRD-2.3: MITM Packet Forwarding

**Problem:**  
CutNet can poison ARP caches but doesn't forward intercepted packets. This breaks victim's internet entirely instead of allowing monitoring/optional blocking.

**Requirements:**
- [ ] Enable IP forwarding at system level
- [ ] Intercept packets between victim and router
- [ ] Forward legitimate traffic (transparent MITM)
- [ ] Optionally filter/block specific traffic
- [ ] Support packet inspection hooks
- [ ] Log forwarded traffic statistics
- [ ] Handle both TCP and UDP
- [ ] Support connection tracking for stateful forwarding

**Technical Approach:**

**System IP Forwarding:**
```rust
// network/forwarding.rs
#[cfg(target_os = "linux")]
pub fn enable_ip_forwarding() -> Result<(), ForwardingError> {
    // Enable IPv4 forwarding via sysctl
    Command::new("sysctl")
        .args(["-w", "net.ipv4.ip_forward=1"])
        .status()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn enable_ip_forwarding() -> Result<(), ForwardingError> {
    Command::new("sysctl")
        .args(["-w", "net.inet.ip.forwarding=1"])
        .status()?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn enable_ip_forwarding() -> Result<(), ForwardingError> {
    // Enable via registry or netsh
    Command::new("reg")
        .args([
            "add", r"HKLM\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
            "/v", "IPEnableRouter", "/t", "REG_DWORD", "/d", "1", "/f"
        ])
        .status()?;
    Ok(())
}
```

**Packet Forwarding Engine:**
```rust
// forwarder.rs
use pnet_packet::{ipv4::Ipv4Packet, tcp::TcpPacket, udp::UdpPacket};

pub struct PacketForwarder {
    victim_ip: Ipv4Addr,
    router_ip: Ipv4Addr,
    forwarding_rules: RwLock<Vec<ForwardingRule>>,
    stats: RwLock<ForwardStats>,
}

#[derive(Clone)]
pub struct ForwardingRule {
    pub protocol: Protocol,
    pub port: Option<u16>,
    pub action: ForwardAction,
}

pub enum ForwardAction {
    Allow,      // Forward normally
    Block,      // Drop packet
    Log,        // Log then forward
    Modify,     // Apply packet modifications
}

pub async fn forward_packets(
    interface: NetworkInterface,
    victim: Device,
    router: Device,
) -> Result<(), ForwardingError> {
    let (mut tx, mut rx) = create_raw_socket(&interface)?;
    
    loop {
        match rx.next() {
            Ok(packet) => {
                if let Some(ipv4) = Ipv4Packet::new(&packet) {
                    handle_ip_packet(&ipv4, &victim, &router, &mut tx).await?;
                }
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(1)).await,
        }
    }
}

async fn handle_ip_packet(
    packet: &Ipv4Packet,
    victim: &Device,
    router: &Device,
    tx: &mut Box<dyn DataLinkSender>,
) -> Result<(), ForwardingError> {
    let src_ip = packet.get_source();
    let dst_ip = packet.get_destination();
    
    // Determine direction
    let is_from_victim = src_ip.to_string() == victim.ip;
    let is_to_victim = dst_ip.to_string() == victim.ip;
    
    if !is_from_victim && !is_to_victim {
        return Ok(()); // Not our target, skip
    }
    
    // Check forwarding rules
    let rules = FORWARDING_RULES.read().await;
    for rule in rules.iter() {
        if rule.matches(packet) {
            match rule.action {
                ForwardAction::Block => return Ok(()), // Drop silently
                ForwardAction::Log => log_packet(packet),
                ForwardAction::Modify => modify_packet(packet)?,
                ForwardAction::Allow => {}
            }
        }
    }
    
    // Fix checksums after any modifications
    let mut packet_vec = packet.raw().to_vec();
    recalculate_checksums(&mut packet_vec);
    
    // Forward to correct destination
    let dest_mac = if is_from_victim {
        router.mac.clone()
    } else {
        victim.mac.clone()
    };
    
    // Update Ethernet destination and send
    update_ethernet_dest(&mut packet_vec, &dest_mac)?;
    tx.send_to(&packet_vec, Some(interface))?;
    
    // Update stats
    let mut stats = FORWARD_STATS.write().await;
    stats.packets_forwarded += 1;
    stats.bytes_forwarded += packet_vec.len() as u64;
    
    Ok(())
}
```

**Connection Tracking:**
```rust
// conntrack.rs
pub struct ConnectionTracker {
    connections: RwLock<HashMap<ConnectionKey, ConnectionState>>,
}

#[derive(Hash, Eq, PartialEq)]
pub struct ConnectionKey {
    pub src_ip: Ipv4Addr,
    pub dst_ip: Ipv4Addr,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8, // TCP=6, UDP=17
}

pub struct ConnectionState {
    pub created_at: u64,
    pub last_seen: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub state: TcpState, // ESTABLISHED, SYN_SENT, etc.
}

impl ConnectionTracker {
    pub async fn track_packet(&self, packet: &Ipv4Packet) {
        if let Some(tcp) = packet.get_tcp_header() {
            let key = ConnectionKey::from_packet(packet);
            let mut connections = self.connections.write().await;
            
            let conn = connections.entry(key).or_insert_with(|| ConnectionState::new());
            conn.last_seen = current_timestamp();
            
            // Update TCP state machine
            conn.update_state(tcp);
        }
    }
    
    pub async fn cleanup_stale(&self) {
        let mut connections = self.connections.write().await;
        let now = current_timestamp();
        let timeout = 300; // 5 minutes
        
        connections.retain(|_, state| now - state.last_seen < timeout);
    }
}
```

**Success Criteria:**
- Enable IP forwarding, verify via `sysctl net.ipv4.ip_forward`
- Victim maintains internet connectivity while being poisoned
- Packet forwarding adds <10ms latency
- Connection tracking handles 100+ concurrent connections
- Stats show accurate packet/byte counts
- Can block specific ports (e.g., block port 80)
- TCP state tracking works correctly

**Files to Create:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/forwarder.rs`
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/conntrack.rs`
- `/Users/encore/Documents/personal/repositories/cutnet/src/components/ForwardingPanel.tsx`

**Files to Modify:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/network/poisoner.rs` (integrate forwarding)
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/ipc/commands.rs` (add forwarding commands)
- `/Users/encore/Documents/personal/repositories/cutnet/src/App.tsx` (add forwarding tab)

---

## PRIORITY 3: SECURITY & STABILITY

### PRD-3.1: Add Privilege Check at Startup (VULN-003)

**Problem:**  
Privilege check happens after UI loads, causing confusing errors when raw socket operations fail.

**Requirements:**
- [ ] Check privileges before main window opens
- [ ] Show platform-specific elevation instructions
- [ ] Offer to restart with elevation (where supported)
- [ ] Run in limited mode if user declines elevation

**Implementation:**
```rust
// main.rs
fn main() {
    let admin_check = check_admin_privileges();
    
    if !admin_check.has_privileges {
        // Show native dialog before main window
        let elevation_result = show_elevation_dialog(&admin_check);
        
        match elevation_result {
            ElevationResult::RetryAsAdmin => {
                restart_as_admin();
                return;
            }
            ElevationResult::ContinueLimited => {
                // Set flag for limited mode
                std::env::set_var("CUTNET_LIMITED_MODE", "1");
            }
            ElevationResult::Exit => {
                std::process::exit(0);
            }
        }
    }
    
    // Continue with normal app startup
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running CutNet");
}
```

**Files to Modify:**
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/main.rs`
- `/Users/encore/Documents/personal/repositories/cutnet/src-tauri/src/ipc/commands.rs` (export check function)

---

### PRD-3.2: Implement State Cleanup (BUG-002)

**Problem:**  
Global static state has no cleanup mechanism, causing memory leaks and stale state.

**Requirements:**
- [ ] Add cleanup hooks for all global state
- [ ] Clear state on app shutdown
- [ ] Provide reset function for testing
- [ ] Handle panics gracefully

**Implementation:**
```rust
// state.rs
pub async fn cleanup_all_state() {
    let mut killer = KILLER_STATE.lock().await;
    killer.unkill_all().await.ok();
    killer.poisoned_devices.clear();
    
    let mut defender = DEFENDER_STATE.write().await;
    defender.is_active = false;
    defender.alerts.clear();
    defender.known_mappings.clear();
    
    log::info!("All state cleaned up");
}

// lib.rs
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                ctrl_c_handler(handle);
            });
            Ok(())
        })
        .on_window_event(|event| {
            if let WindowEvent::CloseRequested { .. } = event.event() {
                // Cleanup before close
                tokio::spawn(async {
                    cleanup_all_state().await;
                });
            }
        })
}
```

---

### PRD-3.3: Add Panic Recovery (BUG-004)

**Requirements:**
- [ ] Catch panics in spawned tasks
- [ ] Log panic details
- [ ] Reset state on panic
- [ ] Notify frontend of recovery

**Implementation:**
```rust
// state.rs
tokio::spawn(async move {
    let result = std::panic::catch_unwind(async {
        crate::network::scanner::arp_scan(&interface_for_scan).await
    }).await;
    
    match result {
        Ok(Ok(devices)) => { /* success */ }
        Ok(Err(e)) => {
            log::error!("ARP scan failed: {}", e);
            emit_scan_completed(&app, 0, false);
        }
        Err(panic_info) => {
            log::error!("ARP scan panicked: {:?}", panic_info);
            emit_error(&app, "Scanner crashed, please retry");
        }
    }
    
    self_arc.lock().await.is_running = false;
});
```

---

### PRD-3.4: Add Confirmation Dialogs (Issue-008)

**Requirements:**
- [ ] Confirm before Kill All
- [ ] Confirm before clearing history
- [ ] Confirm before resetting settings
- [ ] Show affected device count

**Implementation:**
```typescript
// DeviceTable.tsx
const handleKillAll = useCallback(async () => {
    const killableDevices = devices.filter(d => 
        !killStates.get(d.mac)?.is_killed && !d.is_me
    );
    
    const confirmed = await showDialog({
        title: 'Block All Devices?',
        description: `This will block internet access for ${killableDevices.length} devices.`,
        confirmText: 'Block All',
        cancelText: 'Cancel',
        variant: 'destructive',
    });
    
    if (!confirmed) return;
    
    await killAllDevices(killableDevices);
    showToast('success', `Blocked ${killableDevices.length} devices`);
}, [devices, killStates]);
```

---

## VERIFICATION & TESTING PLAN

### Test Categories

| Category | Scope | Priority |
|----------|-------|----------|
| Unit Tests | Individual functions | High |
| Integration Tests | Module interactions | High |
| E2E Tests | Full user flows | Medium |
| Security Tests | Input validation, auth | High |
| Performance Tests | Load, stress | Medium |
| Platform Tests | Windows, macOS, Linux | High |

### Unit Test Requirements

```rust
// scanner_tests.rs
#[test]
fn test_arp_scan_sync_barrier() {
    // Verify receiver ready before first send
}

#[test]
fn test_validate_mac_broadcast() {
    assert!(validate_unicast_mac("FF:FF:FF:FF:FF:FF").is_err());
}

#[test]
fn test_validate_mac_multicast() {
    assert!(validate_unicast_mac("01:00:5E:00:00:01").is_err());
}

// error_tests.rs
#[test]
fn test_error_code_mapping() {
    let err = NetworkError::PermissionDenied("raw_socket".to_string());
    let api_err = ApiError::from(err);
    assert!(matches!(api_err.code, ErrorCode::PermissionDenied { .. }));
    assert_eq!(api_err.retryable, false);
}
```

### Integration Test Requirements

1. **ARP Scan Flow**
   - Start scan → Discover devices → Verify device list matches network
   - Test with 1, 10, 50 devices

2. **Kill/Restore Flow**
   - Kill device → Verify offline (ping fails)
   - Restore device → Verify online (ping succeeds)
   - Test with multiple simultaneous kills

3. **Bandwidth Throttling**
   - Set 1 MB/s limit → Run speed test → Verify capped
   - Remove limit → Run speed test → Verify uncapped

4. **Scheduling**
   - Create schedule → Wait for trigger time → Verify action executed
   - Disable schedule → Wait for trigger time → Verify no action

### E2E Test Requirements

1. **Full User Journey**
   - Launch app → Grant privileges → Scan network → Kill device → Restore → Exit
   - Verify state cleanup on exit

2. **Cross-Platform**
   - Run identical tests on Windows, macOS, Linux
   - Verify consistent behavior

3. **Stress Test**
   - 50+ device discoveries
   - 20+ simultaneous kills
   - 24-hour continuous operation

### Security Test Requirements

1. **Input Validation**
   - Malformed MAC addresses (too short, too long, invalid chars)
   - Malformed IP addresses
   - SQL injection in device names
   - Path traversal in file operations

2. **Privilege Escalation**
   - Verify no privilege escalation possible through IPC
   - Verify raw socket operations fail gracefully without privileges

---

## APPENDIX: IMPLEMENTATION TIMELINE

| Week | Tasks | Deliverables |
|------|-------|--------------|
| 1 | PRD-1.1, PRD-1.2, PRD-1.3 | Race condition fix, UI state fix, MAC validation |
| 2 | PRD-1.4, PRD-1.5, PRD-3.1 | Error types, update check, privilege check |
| 3 | PRD-3.2, PRD-3.3, PRD-3.4 | State cleanup, panic recovery, confirmations |
| 4 | PRD-2.1 (Bandwidth) - Phase 1 | Linux/macOS bandwidth limiting |
| 5 | PRD-2.1 (Bandwidth) - Phase 2 | Windows support, UI integration |
| 6 | PRD-2.2 (Scheduling) | Scheduler service, UI, persistence |
| 7 | PRD-2.3 (MITM) - Phase 1 | IP forwarding, packet forwarding |
| 8 | PRD-2.3 (MITM) - Phase 2 | Connection tracking, filtering |
| 9 | Testing & QA | All test categories, bug fixes |
| 10 | Release Prep | Documentation, changelog, version bump |

**Total Estimated Duration:** 10 weeks

---

**PRD Version:** 1.0  
**Approved By:** [Pending]  
**Implementation Start Date:** April 5, 2026
