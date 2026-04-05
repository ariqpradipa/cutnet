//! Tauri command handlers for CutNet IPC

use crate::ipc::error::{ApiError, ApiResult};
use crate::ipc::events::*;
use crate::ipc::state::{KillerState, ScannerState};
use crate::network::NetworkInterface;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

/// Target device for kill operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTarget {
    pub ip: String,
    pub mac: String,
}

/// System information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub platform: String,
    pub version: String,
    pub has_admin_privileges: bool,
    pub hostname: String,
}

fn map_error(e: crate::network::types::NetworkError) -> ApiError {
    ApiError::from(e)
}

/// Get all network interfaces available on the system
#[tauri::command]
pub async fn get_interfaces() -> ApiResult<Vec<NetworkInterface>> {
    log::info!("Getting network interfaces");

    match get_if_addrs::get_if_addrs() {
        Ok(if_addrs) => {
            let mut interfaces = Vec::new();

            for iface in if_addrs {
                if iface.is_loopback() {
                    continue;
                }

                if let get_if_addrs::IfAddr::V4(v4_addr) = iface.addr {
                    let ip = v4_addr.ip.to_string();
                    let netmask = v4_addr.netmask.to_string();
                    let broadcast = v4_addr.broadcast.map(|b| b.to_string()).unwrap_or_else(|| {
                        format!("{}", ip)
                    });

                    let mac = crate::network::get_interface_mac(&iface.name)
                        .unwrap_or_else(|_| "00:00:00:00:00:00".to_string());

                    interfaces.push(NetworkInterface::new(
                        iface.name,
                        ip,
                        mac,
                        broadcast,
                        netmask,
                    ));
                }
            }

            log::info!("Found {} network interfaces", interfaces.len());
            Ok(interfaces)
        }
        Err(e) => {
            log::error!("Failed to get interfaces: {}", e);
            Err(ApiError::new(
                crate::ipc::error::ErrorCode::IoError,
                "Failed to get network interfaces"
            ).with_details(e.to_string()))
        }
    }
}

/// Start ARP scan on the specified interface
#[tauri::command]
pub async fn start_arp_scan(
    interface_name: String,
    scanner: State<'_, ScannerState>,
    killer: State<'_, KillerState>,
    app: AppHandle,
) -> ApiResult<()> {
    log::info!("Starting ARP scan on interface: {}", interface_name);

    let mut scanner_lock = scanner.lock().await;

    if scanner_lock.is_running() {
        return Err(ApiError::new(
            crate::ipc::error::ErrorCode::ScanError,
            "Scan already in progress"
        ));
    }

    match scanner_lock.start_arp_scan(interface_name.clone(), app, scanner.inner().clone()) {
        Ok(()) => {
            log::info!("ARP scan started successfully");

            let router = crate::network::scanner::get_current_interface()
                .ok()
                .map(|iface| crate::network::Device::new(&iface.ip, &iface.mac).as_router());

            if let Some(router_device) = router {
                let mut killer_lock = killer.lock().await;
                killer_lock.set_interface_and_router(interface_name, router_device);
            }

            Ok(())
        }
        Err(e) => {
            log::error!("Failed to start ARP scan: {}", e);
            Err(map_error(e))
        }
    }
}

/// Start ping scan on the specified interface
#[tauri::command]
pub async fn start_ping_scan(
    interface_name: String,
    scanner: State<'_, ScannerState>,
    killer: State<'_, KillerState>,
    app: AppHandle,
) -> ApiResult<()> {
    log::info!("Starting ping scan on interface: {}", interface_name);

    let mut scanner_lock = scanner.lock().await;

    if scanner_lock.is_running() {
        return Err(ApiError::new(
            crate::ipc::error::ErrorCode::ScanError,
            "Scan already in progress"
        ));
    }

    match scanner_lock.start_ping_scan(interface_name.clone(), app, scanner.inner().clone()) {
        Ok(()) => {
            log::info!("Ping scan started successfully");

            let router = crate::network::scanner::get_current_interface()
                .ok()
                .map(|iface| crate::network::Device::new(&iface.ip, &iface.mac).as_router());

            if let Some(router_device) = router {
                let mut killer_lock = killer.lock().await;
                killer_lock.set_interface_and_router(interface_name, router_device);
            }

            Ok(())
        }
        Err(e) => {
            log::error!("Failed to start ping scan: {}", e);
            Err(map_error(e))
        }
    }
}

/// Stop any active scan
#[tauri::command]
pub async fn stop_scan(scanner: State<'_, ScannerState>) -> ApiResult<()> {
    log::info!("Stopping scan");

    let mut scanner_lock = scanner.lock().await;
    scanner_lock.stop_scan();

    log::info!("Scan stopped");
    Ok(())
}

/// Kill (block internet access for) a specific device
#[tauri::command]
pub async fn kill_device(
    ip: String,
    mac: String,
    killer: State<'_, KillerState>,
    app: AppHandle,
) -> ApiResult<()> {
    log::info!("Killing device: {} ({})", ip, mac);

    let mut killer_lock = killer.lock().await;

    match killer_lock.kill_device(ip.clone(), mac.clone()).await {
        Ok(()) => {
            emit_device_killed(&app, ip, mac);
            log::info!("Device killed successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to kill device: {}", e);
            Err(map_error(e))
        }
    }
}

/// Restore internet access for a specific device
#[tauri::command]
pub async fn unkill_device(
    ip: String,
    mac: String,
    killer: State<'_, KillerState>,
    app: AppHandle,
) -> Result<(), String> {
    log::info!("Restoring device: {} ({})", ip, mac);

    let mut killer_lock = killer.lock().await;

    match killer_lock.unkill_device(ip.clone(), mac.clone()).await {
        Ok(()) => {
            emit_device_restored(&app, ip, mac);
            log::info!("Device restored successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to restore device: {}", e);
            Err(e.to_string())
        }
    }
}

/// Kill multiple devices at once
#[tauri::command]
pub async fn kill_all_devices(
    devices: Vec<DeviceTarget>,
    killer: State<'_, KillerState>,
    app: AppHandle,
) -> Result<(), String> {
    log::info!("Killing {} devices", devices.len());

    let mut killer_lock = killer.lock().await;

    for device in devices {
        match killer_lock.kill_device(device.ip.clone(), device.mac.clone()).await {
            Ok(()) => {
                emit_device_killed(&app, device.ip, device.mac);
            }
            Err(e) => {
                log::error!("Failed to kill device {}: {}", device.ip, e);
            }
        }
    }

    log::info!("Batch kill operation completed");
    Ok(())
}

/// Restore all killed devices
#[tauri::command]
pub async fn unkill_all_devices(
    killer: State<'_, KillerState>,
    app: AppHandle,
) -> Result<(), String> {
    log::info!("Restoring all devices");

    let mut killer_lock = killer.lock().await;

    match killer_lock.unkill_all().await {
        Ok(devices) => {
            for (ip, mac) in devices {
                emit_device_restored(&app, ip, mac);
            }
            log::info!("All devices restored successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to restore all devices: {}", e);
            Err(e.to_string())
        }
    }
}

/// Get MAC address for a network interface
#[tauri::command]
pub async fn get_mac_address(interface: String) -> Result<String, String> {
    log::info!("Getting MAC address for interface: {}", interface);

    crate::network::get_mac_address(&interface).map_err(|e| e.to_string())
}

/// Set MAC address for a network interface
#[tauri::command]
pub async fn set_mac_address(
    interface: String,
    new_mac: String,
    app: AppHandle,
) -> Result<(), String> {
    log::info!("Setting MAC address for {} to {}", interface, new_mac);

    match crate::network::set_mac_address(&interface, &new_mac) {
        Ok(()) => {
            emit_mac_address_changed(&app, interface, new_mac);
            log::info!("MAC address set successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to set MAC address: {}", e);
            Err(e.to_string())
        }
    }
}

/// Clone MAC address from one interface to another
#[tauri::command]
pub async fn clone_mac_address(from: String, to: String, app: AppHandle) -> Result<(), String> {
    log::info!("Cloning MAC from {} to {}", from, to);

    match crate::network::clone_mac(&from, &to) {
        Ok(()) => {
            let source_mac = crate::network::get_mac_address(&from).unwrap_or_default();
            emit_mac_address_changed(&app, to, source_mac);
            log::info!("MAC cloned successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to clone MAC: {}", e);
            Err(e.to_string())
        }
    }
}

/// Check if the application has administrative privileges
#[tauri::command]
pub async fn check_admin_privileges() -> Result<bool, String> {
    log::info!("Checking admin privileges");

    #[cfg(target_os = "linux")]
    {
        let uid = unsafe { libc::getuid() };
        Ok(uid == 0)
    }

    #[cfg(target_os = "macos")]
    {
        let uid = unsafe { libc::getuid() };
        Ok(uid == 0)
    }

    #[cfg(target_os = "windows")]
    {
        match is_windows_admin() {
            Ok(is_admin) => Ok(is_admin),
            Err(e) => {
                log::error!("Failed to check admin privileges: {}", e);
                Err(format!("Failed to check privileges: {}", e))
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err("Platform not supported".to_string())
    }
}

/// Get system information
#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    log::info!("Getting system info");

    let platform = std::env::consts::OS.to_string();
    let version = get_os_version().await?;
    let has_admin_privileges = check_admin_privileges().await.unwrap_or(false);
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(SystemInfo {
        platform,
        version,
        has_admin_privileges,
        hostname,
    })
}

#[cfg(target_os = "windows")]
fn is_windows_admin() -> Result<bool, String> {
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::OpenProcessToken;

    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err("Failed to open process token".to_string());
        }

        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;

        let result = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            size,
            &mut size,
        );

        CloseHandle(token);

        if result == 0 {
            return Err("Failed to get token information".to_string());
        }

        Ok(elevation.TokenIsElevated != 0)
    }
}

/// Get OS version
async fn get_os_version() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        match std::fs::read_to_string("/etc/os-release") {
            Ok(content) => {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        return Ok(line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string());
                    }
                }
                Ok("Linux".to_string())
            }
            Err(_) => Ok("Linux".to_string()),
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        match Command::new("sw_vers").arg("-productVersion").output() {
            Ok(output) => {
                if output.status.success() {
                    Ok(format!("macOS {}", String::from_utf8_lossy(&output.stdout).trim()))
                } else {
                    Ok("macOS".to_string())
                }
            }
            Err(_) => Ok("macOS".to_string()),
        }
    }

    #[cfg(target_os = "windows")]
    {
        Ok(format!("Windows {}", sysinfo::System::kernel_version().unwrap_or_else(|| "Unknown".to_string())))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Ok("Unknown".to_string())
    }
}

/// Start ARP Defender monitoring
#[tauri::command]
pub async fn start_defender(app: AppHandle) -> Result<(), String> {
    let interface = crate::network::get_current_interface().map_err(|e| e.to_string())?;
    crate::network::defender::start_defender_monitoring(&interface.name, &app)
        .await
        .map_err(|e| e.to_string())
}

/// Stop ARP Defender monitoring
#[tauri::command]
pub async fn stop_defender() -> Result<(), String> {
    crate::network::defender::stop_defender_monitoring()
        .await
        .map_err(|e| e.to_string())
}

/// Get ARP Defender alerts
#[tauri::command]
pub async fn get_defender_alerts() -> Result<Vec<crate::network::defender::SpoofAlert>, String> {
    Ok(crate::network::defender::get_defender_alerts().await)
}

/// Clear ARP Defender alerts
#[tauri::command]
pub async fn clear_defender_alerts() -> Result<(), String> {
    crate::network::defender::clear_defender_alerts().await;
    Ok(())
}

/// Check if defender is active
#[tauri::command]
pub async fn is_defender_active() -> Result<bool, String> {
    Ok(crate::network::defender::is_defender_active().await)
}

/// Add MAC to whitelist
#[tauri::command]
pub async fn add_whitelist_entry(mac: String, label: Option<String>) -> Result<(), String> {
    crate::network::whitelist::add_entry(mac, label)
        .await
        .map_err(|e| e.to_string())
}

/// Remove MAC from whitelist
#[tauri::command]
pub async fn remove_whitelist_entry(mac: String) -> Result<bool, String> {
    Ok(crate::network::whitelist::remove_entry(&mac).await)
}

/// Get whitelist entries
#[tauri::command]
pub async fn get_whitelist_entries() -> Result<Vec<crate::network::whitelist::WhitelistEntry>, String> {
    Ok(crate::network::whitelist::get_entries().await)
}

/// Set whitelist protection
#[tauri::command]
pub async fn set_whitelist_protect(enabled: bool) -> Result<(), String> {
    crate::network::whitelist::set_protect_enabled(enabled)
        .await
        .map_err(|e| e.to_string())
}

/// Check if MAC is whitelisted
#[tauri::command]
pub async fn is_whitelisted(mac: String) -> Result<bool, String> {
    Ok(crate::network::whitelist::is_whitelisted(&mac).await)
}

/// Flush ARP cache
#[tauri::command]
pub async fn flush_arp_cache_cmd() -> Result<(), String> {
    crate::network::utils::flush_arp_cache()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_history() -> Result<Vec<crate::network::history::DeviceSession>, String> {
    Ok(crate::network::history::get_sessions().await)
}

#[tauri::command]
pub async fn clear_history() -> Result<(), String> {
    crate::network::history::clear_history().await;
    Ok(())
}

#[tauri::command]
pub async fn set_device_custom_name(ip: String, name: String) -> Result<(), String> {
    crate::network::device_names::set_custom_name(ip, name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_custom_device_names() -> Result<std::collections::HashMap<String, String>, String> {
    Ok(crate::network::device_names::get_all_names().await)
}

/// Set bandwidth limit for a device
#[tauri::command]
pub async fn set_bandwidth_limit(
    mac: String,
    download_kbps: Option<u32>,
    upload_kbps: Option<u32>,
) -> Result<(), String> {
    log::info!("Setting bandwidth limit for MAC {}: download={:?}, upload={:?}", mac, download_kbps, upload_kbps);

    crate::network::add_limit_and_persist(&mac, download_kbps, upload_kbps)
        .await
        .map_err(|e| e.to_string())
}

/// Remove bandwidth limit for a device
#[tauri::command]
pub async fn remove_bandwidth_limit(mac: String) -> Result<(), String> {
    log::info!("Removing bandwidth limit for MAC {}", mac);

    crate::network::remove_limit_and_persist(&mac)
        .await
        .map_err(|e| e.to_string())
}

/// Get all bandwidth limits
#[tauri::command]
pub async fn get_bandwidth_limits() -> Result<Vec<crate::network::bandwidth::BandwidthLimit>, String> {
    let controller = crate::network::get_bandwidth_controller();

    if let Some(ctrl) = controller {
        Ok(ctrl.get_limits().await)
    } else {
        Ok(crate::network::get_persisted_limits().await.unwrap_or_default())
    }
}

/// Get bandwidth statistics for a device
#[tauri::command]
pub async fn get_bandwidth_stats(mac: String) -> Result<crate::network::bandwidth::BandwidthStats, String> {
    let controller = crate::network::get_bandwidth_controller()
        .ok_or("Bandwidth controller not initialized")?;

    controller.get_stats(&mac).await.map_err(|e| e.to_string())
}
