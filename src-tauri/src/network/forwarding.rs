//! System IP forwarding configuration
//!
//! This module handles enabling/disabling IP forwarding at the system level
//! for Linux, macOS, and Windows platforms.

use crate::network::types::{NetworkError, Result};

/// Check if IP forwarding is currently enabled on the system
pub async fn is_ip_forwarding_enabled() -> Result<bool> {
    #[cfg(target_os = "linux")]
    {
        check_linux_ip_forwarding().await
    }

    #[cfg(target_os = "macos")]
    {
        check_macos_ip_forwarding().await
    }

    #[cfg(target_os = "windows")]
    {
        check_windows_ip_forwarding().await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(NetworkError::PlatformNotSupported(
            "IP forwarding check not supported on this platform".to_string(),
        ))
    }
}

/// Enable IP forwarding on the system
pub async fn enable_ip_forwarding() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        set_linux_ip_forwarding(true).await
    }

    #[cfg(target_os = "macos")]
    {
        set_macos_ip_forwarding(true).await
    }

    #[cfg(target_os = "windows")]
    {
        set_windows_ip_forwarding(true).await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(NetworkError::PlatformNotSupported(
            "IP forwarding enable not supported on this platform".to_string(),
        ))
    }
}

/// Disable IP forwarding on the system
pub async fn disable_ip_forwarding() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        set_linux_ip_forwarding(false).await
    }

    #[cfg(target_os = "macos")]
    {
        set_macos_ip_forwarding(false).await
    }

    #[cfg(target_os = "windows")]
    {
        set_windows_ip_forwarding(false).await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(NetworkError::PlatformNotSupported(
            "IP forwarding disable not supported on this platform".to_string(),
        ))
    }
}

#[cfg(target_os = "linux")]
async fn check_linux_ip_forwarding() -> Result<bool> {
    use std::process::Command;

    let output = Command::new("sysctl")
        .args(["-n", "net.ipv4.ip_forward"])
        .output()
        .map_err(|e| NetworkError::IoError(e))?;

    if !output.status.success() {
        return Err(NetworkError::PermissionDenied(
            "Failed to check IP forwarding status (requires root)".to_string(),
        ));
    }

    let value = String::from_utf8_lossy(&output.stdout);
    Ok(value.trim() == "1")
}

#[cfg(target_os = "linux")]
async fn set_linux_ip_forwarding(enabled: bool) -> Result<()> {
    use std::process::Command;

    let value = if enabled { "1" } else { "0" };

    let output = Command::new("sysctl")
        .args(["-w", &format!("net.ipv4.ip_forward={}", value)])
        .output()
        .map_err(|e| NetworkError::IoError(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NetworkError::PermissionDenied(format!(
            "Failed to {} IP forwarding: {} (requires root)",
            if enabled { "enable" } else { "disable" },
            stderr
        )));
    }

    log::info!("IP forwarding {} on Linux", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

#[cfg(target_os = "macos")]
async fn check_macos_ip_forwarding() -> Result<bool> {
    use std::process::Command;

    let output = Command::new("sysctl")
        .args(["-n", "net.inet.ip.forwarding"])
        .output()
        .map_err(|e| NetworkError::IoError(e))?;

    if !output.status.success() {
        return Err(NetworkError::PermissionDenied(
            "Failed to check IP forwarding status (requires root)".to_string(),
        ));
    }

    let value = String::from_utf8_lossy(&output.stdout);
    Ok(value.trim() == "1")
}

#[cfg(target_os = "macos")]
async fn set_macos_ip_forwarding(enabled: bool) -> Result<()> {
    use std::process::Command;

    let value = if enabled { "1" } else { "0" };

    let output = Command::new("sysctl")
        .args(["-w", &format!("net.inet.ip.forwarding={}", value)])
        .output()
        .map_err(|e| NetworkError::IoError(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NetworkError::PermissionDenied(format!(
            "Failed to {} IP forwarding: {} (requires root)",
            if enabled { "enable" } else { "disable" },
            stderr
        )));
    }

    log::info!("IP forwarding {} on macOS", if enabled { "enabled" } else { "disabled" });
    Ok(())
}

#[cfg(target_os = "windows")]
async fn check_windows_ip_forwarding() -> Result<bool> {
    use std::process::Command;

    let output = Command::new("reg")
        .args([
            "query",
            "HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters",
            "/v",
            "IPEnableRouter",
        ])
        .output()
        .map_err(|e| NetworkError::IoError(e))?;

    if !output.status.success() {
        return Ok(false);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("0x1"))
}

#[cfg(target_os = "windows")]
async fn set_windows_ip_forwarding(enabled: bool) -> Result<()> {
    use std::process::Command;

    let value = if enabled { "1" } else { "0" };

    let output = Command::new("reg")
        .args([
            "add",
            "HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters",
            "/v",
            "IPEnableRouter",
            "/t",
            "REG_DWORD",
            "/d",
            value,
            "/f",
        ])
        .output()
        .map_err(|e| NetworkError::IoError(e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NetworkError::PermissionDenied(format!(
            "Failed to {} IP forwarding: {} (requires admin)",
            if enabled { "enable" } else { "disable" },
            stderr
        )));
    }

    log::info!(
        "IP forwarding {} on Windows (requires service restart to take effect)",
        if enabled { "enabled" } else { "disabled" }
    );

    Ok(())
}

/// Get the sysctl command path for the current platform
pub fn get_sysctl_path() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "/proc/sys/net/ipv4/ip_forward"
    }

    #[cfg(target_os = "macos")]
    {
        "net.inet.ip.forwarding"
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        ""
    }
}