use std::process::Command;

use crate::network::types::{NetworkError, Result};
use crate::network::utils::{format_mac, is_valid_mac, parse_mac};

pub fn get_mac_address(interface: &str) -> Result<String> {
    let interfaces = pnet_datalink::interfaces();

    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface.to_string()))?;

    let mac = interface.mac.ok_or_else(|| {
        NetworkError::MacAddressError(format!("No MAC address on interface {}", interface))
    })?;

    Ok(format_mac(&mac.octets()))
}

pub fn set_mac_address(interface: &str, new_mac: &str) -> Result<()> {
    if !is_valid_mac(new_mac) {
        return Err(NetworkError::InvalidMacAddress(new_mac.to_string()));
    }

    let formatted_mac = format_mac(&parse_mac(new_mac)?);

    #[cfg(target_os = "macos")]
    {
        set_mac_macos(interface, &formatted_mac)
    }

    #[cfg(target_os = "linux")]
    {
        set_mac_linux(interface, &formatted_mac)
    }

    #[cfg(target_os = "windows")]
    {
        Err(NetworkError::PlatformNotSupported(
            "MAC address changes on Windows require registry modifications and a reboot. \
             Please use Device Manager or third-party tools."
                .to_string(),
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(NetworkError::PlatformNotSupported(
            "MAC address changes not supported on this platform".to_string(),
        ))
    }
}

#[cfg(target_os = "macos")]
fn set_mac_macos(interface: &str, new_mac: &str) -> Result<()> {
    let output = Command::new("ifconfig")
        .args([interface, "lladdr", new_mac])
        .output()
        .map_err(|e| NetworkError::MacSetError(format!("Failed to run ifconfig: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NetworkError::MacSetError(format!(
            "ifconfig failed: {}",
            stderr
        )));
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn set_mac_linux(interface: &str, new_mac: &str) -> Result<()> {
    let down_output = Command::new("ip")
        .args(["link", "set", "dev", interface, "down"])
        .output()
        .map_err(|e| NetworkError::MacSetError(format!("Failed to run ip link down: {}", e)))?;

    if !down_output.status.success() {
        let stderr = String::from_utf8_lossy(&down_output.stderr);
        return Err(NetworkError::MacSetError(format!(
            "ip link down failed: {}",
            stderr
        )));
    }

    let addr_output = Command::new("ip")
        .args(["link", "set", "dev", interface, "address", new_mac])
        .output()
        .map_err(|e| NetworkError::MacSetError(format!("Failed to run ip link address: {}", e)))?;

    if !addr_output.status.success() {
        let stderr = String::from_utf8_lossy(&addr_output.stderr);
        return Err(NetworkError::MacSetError(format!(
            "ip link address failed: {}",
            stderr
        )));
    }

    let up_output = Command::new("ip")
        .args(["link", "set", "dev", interface, "up"])
        .output()
        .map_err(|e| NetworkError::MacSetError(format!("Failed to run ip link up: {}", e)))?;

    if !up_output.status.success() {
        let stderr = String::from_utf8_lossy(&up_output.stderr);
        return Err(NetworkError::MacSetError(format!(
            "ip link up failed: {}",
            stderr
        )));
    }

    Ok(())
}

pub fn clone_mac(from_interface: &str, to_interface: &str) -> Result<()> {
    let source_mac = get_mac_address(from_interface)?;
    set_mac_address(to_interface, &source_mac)?;
    Ok(())
}

pub fn get_original_mac(interface: &str) -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("ifconfig")
            .arg(interface)
            .output()
            .map_err(|e| NetworkError::MacAddressError(format!("Failed to run ifconfig: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Some(line) = stdout.lines().find(|l| l.contains("ether")) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(&mac) = parts.iter().nth(1) {
                if is_valid_mac(mac) {
                    return Ok(mac.to_string());
                }
            }
        }

        Err(NetworkError::MacAddressError(
            "Could not parse MAC address from ifconfig output".to_string(),
        ))
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("ip")
            .args(["link", "show", interface])
            .output()
            .map_err(|e| NetworkError::MacAddressError(format!("Failed to run ip: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Some(line) = stdout.lines().find(|l| l.contains("link/ether")) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(&mac) = parts.iter().nth(1) {
                if is_valid_mac(mac) {
                    return Ok(mac.to_string());
                }
            }
        }

        Err(NetworkError::MacAddressError(
            "Could not parse MAC address from ip output".to_string(),
        ))
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        let output = Command::new("getmac")
            .args(["/v", "/fo", "csv"])
            .output()
            .map_err(|e| NetworkError::MacAddressError(format!("Failed to run getmac: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 3 {
                let adapter_name = parts[0].trim().trim_matches('"');
                let mac = parts[2].trim().trim_matches('"');

                if adapter_name
                    .to_lowercase()
                    .contains(&interface.to_lowercase())
                    || adapter_name.to_lowercase().contains("ethernet")
                {
                    let normalized = mac.replace("-", ":").to_lowercase();
                    if is_valid_mac(&normalized) {
                        return Ok(normalized);
                    }
                }
            }
        }

        Err(NetworkError::MacAddressError(
            "Could not find MAC address for interface".to_string(),
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(NetworkError::PlatformNotSupported(
            "MAC address retrieval not supported on this platform".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_mac_in_parse() {
        assert!(is_valid_mac("aa:bb:cc:dd:ee:ff"));
        assert!(!is_valid_mac("invalid"));
    }
}
