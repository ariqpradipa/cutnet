# CutNet Code Review Report

**Repository:** cutnet  
**Type:** Tauri/Rust/React Application (NetCut Clone)  
**Review Date:** April 5, 2026  
**Version:** 0.1.0  

---

## Executive Summary

CutNet is a network administration tool built with Tauri 2, combining a Rust backend for raw network operations with a React/TypeScript frontend. The application implements ARP scanning, ARP poisoning for device blocking, and ARP spoofing defense capabilities.

**Overall Assessment:** The codebase shows functional ARP manipulation capabilities but has significant gaps in production readiness. Critical bugs, security vulnerabilities, and architectural debt need immediate attention before deployment.

**Risk Level:** HIGH - Contains security vulnerabilities, race conditions, and unhandled edge cases that could cause system instability or security breaches.

---

## 1. Feature Completeness Matrix

### NetCut Requirements vs CutNet Implementation

| Feature | NetCut Standard | CutNet Status | Gap Analysis |
|---------|----------------|---------------|--------------|
| **Device Discovery** | | | |
| ARP Scanning | ✓ | ✓ Implemented | Functional but with race conditions |
| Ping Sweep | ✓ | ✓ Implemented | Uses system ping command (portable but slow) |
| Real-time Device List | ✓ | ✓ Implemented | No auto-refresh mechanism |
| Hostname Resolution | ✓ | ✗ Missing | DNS lookup exists but unused |
| **Device Control** | | | |
| Block Device (Kill) | ✓ | ✓ Implemented | ARP poisoning functional |
| Unblock Device | ✓ | ✓ Implemented | Restore packets sent |
| Kill All | ✓ | ✓ Implemented | No confirmation dialog |
| Bandwidth Throttling | ✓ | ✗ Missing | UI shows "Coming Soon" |
| Scheduled Kill/Restore | ✓ | ✗ Missing | No timer implementation |
| **Network Analysis** | | | |
| Real-time Bandwidth Monitor | ✓ | ✗ Missing | Placeholder only |
| Traffic Graphs | ✓ | ✗ Missing | Not implemented |
| Connection Status | ✓ | Partial | Shows online/offline only |
| **Protection** | | | |
| ARP Spoofing Detection | ✓ | ✓ Implemented | Basic rate-based detection |
| Self-Protection Mode | ✓ | ✓ Implemented | Defender monitoring |
| Whitelist | ✓ | ✓ Implemented | Functional |
| **UI/UX** | | | |
| Device Icons by Vendor | ✓ | ✓ Implemented | Limited vendor list (Apple-heavy) |
| Custom Device Names | ✓ | ✓ Implemented | Functional |
| Dark Mode | ✓ | ✓ Implemented | Functional |
| Session History | ✓ | ✓ Implemented | Join/leave tracking |

**Implementation Coverage:** ~65% of NetCut features are implemented. Missing bandwidth monitoring, scheduling, and MITM capabilities represent significant gaps.

---

## 2. Critical Bugs

### 2.1 Backend Issues

#### BUG-001: Error Handling Loses Type Information
**Location:** `commands.rs` lines 165-183, 180-208  
**Severity:** HIGH  
**Description:** The `map_error` function converts `NetworkError` to a generic string, losing error type discrimination.

```rust
// Problem code:
fn map_error(e: NetworkError) -> String {
    e.to_string()  // Loses error type information!
}

// Impact: Frontend cannot distinguish between:
// - Permission denied (user fixable)
// - Interface not found (configuration issue)
// - Raw socket error (platform issue)
```

**Fix:** Return structured error types through Tauri:
```rust
#[derive(Serialize)]
struct ApiError {
    code: String,
    message: String,
    retryable: bool,
}
```

---

#### BUG-002: Global Static State Without Cleanup
**Location:** `poisoner.rs` lines 12-14  
**Severity:** HIGH  
**Description:** Global `POISONING_STATE` uses `once_cell::sync::Lazy` with RwLock but has no cleanup mechanism on application shutdown.

```rust
static POISONING_STATE: once_cell::sync::Lazy<RwLock<HashMap<String, PoisoningState>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));
```

**Impact:** 
- Memory leaks on repeated start/stop cycles
- State persists across test runs (makes testing unreliable)
- No way to force-reset poisoning state

**Fix:** Implement proper lifecycle management with cleanup hooks.

---

#### BUG-003: ARP Scan Race Condition
**Location:** `scanner.rs` lines 30-69  
**Severity:** CRITICAL  
**Description:** Channel creation and task spawning has a race condition.

```rust
// Problem: tx/rx created here
let (mut tx, mut rx) = create_arp_channel(&interface)?;

// But these tasks start immediately and compete
let recv_task = tokio::spawn(async move {
    receive_arp_replies(&mut rx, &recv_discovered).await;
});

let send_task = tokio::spawn(async move {
    send_arp_requests(&mut tx, ...).await;
});
```

**Impact:** On high-latency systems, replies may arrive before receiver is ready, causing missed devices.

**Fix:** Ensure receiver is ready before sending first request.

---

#### BUG-004: Scanner Task Spawns Without Panic Recovery
**Location:** `state.rs` lines 217-256, 280-357  
**Severity:** MEDIUM  
**Description:** Scanner spawns tokio tasks but doesn't handle panics or task failures.

```rust
tokio::spawn(async move {
    let result = crate::network::scanner::arp_scan(&interface_for_scan).await;
    // ... no panic handling
});
```

**Impact:** If ARP scan panics, the scanner state remains `is_running = true` indefinitely.

---

#### BUG-005: Defender State Mutation During Read Lock
**Location:** `defender.rs` lines 99-140  
**Severity:** MEDIUM  
**Description:** The defender loop holds a read lock, then acquires write lock inside nested scope.

```rust
loop {
    {
        let state = DEFENDER_STATE.read().await;  // Read lock
        if !state.is_active { break; }
    }  // Lock dropped

    // ... process packet ...

    {
        let mut state = DEFENDER_STATE.write().await;  // Write lock
        // Mutations happen here
    }
}
```

**Impact:** Not a deadlock currently, but fragile pattern. Rate limiting (`last_rate_reset`) is captured by value, not reference.

---

### 2.2 Frontend Issues

#### BUG-006: Kill Toggle Updates UI Before Confirmation
**Location:** `DeviceTable.tsx` lines 256-291  
**Severity:** HIGH  
**Description:** UI state updates optimistically without waiting for backend confirmation.

```typescript
// Problem: State updated before await
setKillState(device.mac, {
    mac: device.mac,
    is_killed: true,
    kill_type: "arp_poison",
});  // This happens BEFORE await killDevice(device)
await killDevice(device);
```

**Impact:** UI shows device as killed even if backend operation fails. User sees false success state.

**Fix:** Move state update after successful response, or implement proper rollback.

---

#### BUG-007: No Error Type Discrimination in IPC
**Location:** `ipc.ts` lines 23-28  
**Severity:** MEDIUM  
**Description:** All IPC errors are passed as generic strings.

```typescript
export async function killDevice(device: Device): Promise<void> {
    await invoke("kill_device", { 
        ip: device.ip,
        mac: device.mac,
    });
    // Any error is just a thrown string
}
```

**Impact:** Frontend cannot provide contextual error messages or recovery suggestions.

---

#### BUG-008: Fake Update Check
**Location:** `SettingsPanel.tsx` lines 388-398  
**Severity:** LOW  
**Description:** Update check is a simulation with hardcoded result.

```typescript
const handleCheckUpdates = useCallback(async () => {
    setIsCheckingUpdates(true);
    await new Promise((resolve) => setTimeout(resolve, 1500));
    setUpdateStatus({ available: false });  // Always false!
    setIsCheckingUpdates(false);
}, []);
```

---

## 3. Security Vulnerabilities

### VULN-001: No MAC Address Input Validation
**Location:** `poisoner.rs` lines 145-174, 176-205  
**Severity:** HIGH  
**Description:** MAC addresses passed to packet construction are not validated before use.

```rust
async fn poison_target(...) -> Result<()> {
    let target_mac = parse_mac_bytes(&target.mac)?;  // Only validates format, not content
    // MAC could be broadcast, multicast, or all zeros
```

**Impact:** Could accidentally poison broadcast/multicast addresses causing network disruption.

---

### VULN-002: No Rate Limiting on ARP Poisoning
**Location:** `poisoner.rs` lines 90-142  
**Severity:** MEDIUM  
**Description:** Poisoning loop sends packets at fixed interval without considering network conditions.

```rust
let mut interval = tokio::time::interval(Duration::from_millis(config.interval_ms));
// Fixed 2-second interval regardless of network load
```

**Impact:** Could flood network on large device lists, causing performance issues.

---

### VULN-003: Privilege Check Happens Too Late
**Location:** `commands.rs` lines 312-344  
**Severity:** MEDIUM  
**Description:** Admin check is a command, not enforced at startup.

**Impact:** User sees UI before knowing they need elevated privileges. Raw socket operations fail mysteriously.

**Fix:** Check privileges at application startup and show warning before main window opens.

---

### VULN-004: Whitelist Check Async Pattern in Sync Context
**Location:** `state.rs` lines 57-62  
**Severity:** LOW  
**Description:** Async whitelist check pattern could have race conditions.

```rust
if is_whitelisted(&mac).await && is_protected(&mac).await {
    // Two separate async calls - state could change between them
```

---

## 4. Architecture Issues

### Issue-001: Global Static State Makes Testing Impossible
**Locations:**
- `poisoner.rs` lines 12-16
- `defender.rs` lines 24-33
- `whitelist.rs` lines 25-27
- `history.rs` lines 35-37

**Description:** Multiple global statics using `Lazy` pattern prevent:
- Parallel test execution
- Clean state between tests
- Dependency injection
- State inspection/mocking

**Recommendation:** Convert to dependency-injected services with proper lifecycle management.

---

### Issue-002: No Structured Error Types
**Location:** Throughout codebase  
**Description:** Heavy use of `String` errors instead of structured error types.

```rust
// Instead of:
Err(format!("Failed to get interfaces: {}", e))

// Should be:
Err(NetworkError::InterfaceError { 
    source: e,
    interface: name 
})
```

---

### Issue-003: No Structured Logging
**Location:** Throughout codebase  
**Description:** Uses basic `log::info!`/`log::error!` without structured fields.

**Recommendation:** Use `tracing` for structured logging with spans and contextual data.

---

### Issue-004: Scanner and Killer State Coupled
**Location:** `commands.rs` lines 74-110, 113-148  
**Description:** Scanner and Killer share interface/router state implicitly.

**Impact:** Cannot scan on one interface while killing on another. Limits flexibility.

---

### Issue-005: No Unit Tests for Critical Network Operations
**Location:** `Cargo.toml` shows no test configuration  
**Description:** Only basic utility tests exist; no tests for:
- ARP packet construction
- Poisoning logic
- Scanning algorithms
- Error handling paths

---

## 5. UI/UX Issues

### Issue-006: Misleading Bandwidth Column
**Location:** `DeviceTable.tsx` lines 581-593  
**Description:** Bandwidth column shows "Coming Soon" but appears as functional UI.

```typescript
<TooltipContent>
    <p>Coming Soon</p>
</TooltipContent>
```

**Recommendation:** Either implement bandwidth monitoring or remove/hide the column.

---

### Issue-007: No Admin Privilege Warning on Launch
**Location:** `App.tsx`  
**Description:** No check for admin privileges before showing main UI.

---

### Issue-008: No Confirmation Dialog for Kill All
**Location:** `DeviceTable.tsx` lines 284-308  
**Description:** Kill All button executes immediately without confirmation.

```typescript
const handleKillAll = useCallback(async () => {
    const killableDevices = devices.filter(d => !killStates.get(d.mac)?.is_killed && !d.is_me);
    // No confirmation dialog!
    await killAllDevices(killableDevices);
```

---

### Issue-009: History Doesn't Distinguish Kicked vs Naturally Disconnected
**Location:** `HistoryPanel.tsx`  
**Description:** History only tracks join/leave, not whether device was killed.

---

## 6. Edge Cases Not Handled

| Edge Case | Status | Risk |
|-----------|--------|------|
| Multiple network interfaces simultaneously | ❌ Not handled | User can only use one interface at a time |
| Interface hotswap (USB ethernet) | ❌ Not handled | Requires app restart to detect new interface |
| Large networks (254+ hosts) | ⚠️ Partial | ARP scan timeout is fixed at 2 seconds |
| IPv6 networks | ❌ Not handled | Hardcoded IPv4 assumptions throughout |
| VLAN tagging | ❌ Not handled | No VLAN awareness in packet construction |
| Devices with multiple IP addresses | ❌ Not handled | Device identity tied to IP, not MAC |
| ARP scan on empty subnet | ⚠️ Partial | Returns empty list but no user feedback |
| Router MAC changes | ❌ Not handled | Poisoning fails silently if router MAC changes |
| Duplicate IP detection | ❌ Not handled | No detection or handling of IP conflicts |

---

## 7. Implementation Priorities

### Priority 1: Critical (Fix Before Release)

1. **BUG-003:** Fix ARP scan race condition
2. **BUG-006:** Fix UI state update before confirmation
3. **VULN-001:** Add MAC address validation before poisoning
4. **BUG-001:** Implement structured error types for IPC
5. **BUG-008:** Remove or implement real update check

### Priority 2: High (Fix in Next Sprint)

1. **BUG-002:** Add cleanup for global static state
2. **BUG-004:** Add panic recovery for spawned tasks
3. **VULN-003:** Check privileges at application startup
4. **Issue-001:** Refactor global state to dependency injection
5. **Issue-008:** Add confirmation dialogs for destructive actions

### Priority 3: Medium (Technical Debt)

1. **Issue-002:** Migrate from String errors to structured types
2. **Issue-003:** Implement structured logging with tracing
3. **BUG-007:** Add error discrimination to IPC layer
4. **Issue-006:** Implement bandwidth monitoring or remove column
5. **Issue-005:** Add unit tests for network operations

### Priority 4: Low (Enhancements)

1. **Issue-009:** Track killed status in history
2. **Edge cases:** Add IPv6 support, VLAN support
3. **Feature:** Implement bandwidth throttling
4. **Feature:** Add scheduled kill/restore timers
5. **Feature:** MITM packet forwarding

---

## 8. Test Plan Requirements

### Unit Tests

| Component | Coverage Needed | Priority |
|-----------|----------------|----------|
| `poisoner.rs` | ARP packet construction, MAC validation | High |
| `scanner.rs` | ARP scan logic, timeout handling | High |
| `defender.rs` | Alert detection, rate limiting | Medium |
| `whitelist.rs` | Add/remove/query operations | Medium |
| `utils.rs` | MAC parsing, vendor lookup | Low (exists) |

### Integration Tests

1. **Full Scan Flow:** Start scan → Discover devices → Stop scan
2. **Kill/Restore Flow:** Kill device → Verify offline → Restore → Verify online
3. **Defender Flow:** Start defender → Simulate spoofing → Verify alert
4. **Privilege Handling:** Run without admin → Verify graceful failure

### E2E Tests

1. **Cross-platform:** Verify on Windows, macOS, Linux
2. **Network conditions:** Large network, empty network, no permission
3. **Stress testing:** 50+ devices killed simultaneously
4. **Long-running:** App open for 24 hours, memory usage stable

### Security Tests

1. **Input validation:** Malformed MAC addresses, invalid IPs
2. **Fuzz testing:** Random packet data
3. **Permission escalation:** Verify no privilege escalation possible

---

## 9. Recommendations

### Immediate Actions

1. **Stop using global static state** - Convert to dependency-injected services
2. **Implement structured error handling** - Replace String errors with typed errors
3. **Add privilege check at startup** - Show warning before UI loads
4. **Fix race conditions** - Ensure proper synchronization in ARP operations

### Architecture Improvements

1. **Separate concerns** - Scanner, Killer, Defender should be independent services
2. **Add event sourcing** - Track all actions for audit trail
3. **Implement proper logging** - Use tracing with structured fields
4. **Add metrics** - Expose Prometheus metrics for monitoring

### Feature Gaps to Address

1. **Bandwidth monitoring** - Either implement or remove placeholder
2. **Hostname resolution** - Wire up existing DNS lookup
3. **IPv6 support** - Modern networks require this
4. **Proper update mechanism** - Implement real update check

---

## Appendix A: Code Smells

1. **Clippy warnings likely** - Code uses patterns that may trigger warnings
2. **Unnecessary clones** - Many `.clone()` calls that may be unnecessary
3. **unwrap() usage** - Several `unwrap()` calls that could panic
4. **String-based APIs** - Many APIs take `String` instead of `&str`
5. **Dead code** - Several `#[allow(dead_code)]` annotations indicate unused code

## Appendix B: Dependencies Audit

| Dependency | Version | Status |
|------------|---------|--------|
| tauri | 2.x | ✓ Current |
| tokio | 1.x | ✓ Current |
| pnet | 0.34 | ✓ Current |
| serde | 1.x | ✓ Current |
| once_cell | 1.20 | ✓ Current |
| thiserror | 1.x | ✓ Current |

All dependencies are up-to-date. No security advisories found.

---

**Report Generated:** April 5, 2026  
**Reviewer:** Sisyphus-Junior  
**Next Review:** After Priority 1 fixes implemented
