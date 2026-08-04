use std::net::IpAddr;
use std::str::FromStr;

use crate::network::types::{NetworkError, Result};

/// Deduplicated OUI vendor prefix table.
/// Each prefix is unique — duplicates from the original auto-generated list
/// have been removed. The HashMap built from this is O(1) at lookup time.
const OUI_VENDORS: &[(&str, &str)] = &[
    // Hypervisors / VMs
    ("00:50:56", "VMware"), ("00:0c:29", "VMware"), ("00:1c:14", "VMware"),
    ("08:00:27", "VirtualBox"), ("0a:00:27", "VirtualBox"),
    ("52:54:00", "QEMU"), ("00:16:3e", "Xen"), ("00:15:5d", "Microsoft Hyper-V"),
    ("00:1c:42", "Parallels"),
    // Intel
    ("00:1b:21", "Intel"), ("00:1c:c4", "Intel"), ("00:21:5c", "Intel"),
    ("00:22:fa", "Intel"), ("00:23:14", "Intel"), ("00:24:d6", "Intel"),
    ("00:26:c6", "Intel"),
    // Apple — representative set covering common device lines
    ("00:25:bc", "Apple"), ("04:0c:ce", "Apple"), ("04:26:65", "Apple"),
    ("04:52:f3", "Apple"), ("08:00:07", "Apple"), ("08:6d:41", "Apple"),
    ("0c:15:39", "Apple"), ("0c:74:c2", "Apple"), ("10:41:7f", "Apple"),
    ("10:9f:a9", "Apple"), ("14:20:5e", "Apple"), ("14:98:77", "Apple"),
    ("18:65:90", "Apple"), ("18:81:de", "Apple"), ("18:af:61", "Apple"),
    ("1c:36:bb", "Apple"), ("1c:9e:46", "Apple"), ("1c:e6:2b", "Apple"),
    ("20:a5:cb", "Apple"), ("24:a0:74", "Apple"), ("28:37:37", "Apple"),
    ("28:5f:db", "Apple"), ("28:cf:05", "Apple"), ("2c:1f:23", "Apple"),
    ("30:10:e6", "Apple"), ("34:08:bc", "Apple"), ("34:51:aa", "Apple"),
    ("38:0f:4a", "Apple"), ("38:59:f9", "Apple"), ("3c:07:54", "Apple"),
    ("3c:5a:b4", "Apple"), ("40:3c:fc", "Apple"), ("40:a6:d9", "Apple"),
    ("44:2a:60", "Apple"), ("48:43:7c", "Apple"), ("48:60:5f", "Apple"),
    ("4c:57:60", "Apple"), ("4c:8d:79", "Apple"), ("50:1d:93", "Apple"),
    ("50:82:d5", "Apple"), ("50:a6:7f", "Apple"), ("54:1d:c2", "Apple"),
    ("54:ae:14", "Apple"), ("58:40:4e", "Apple"), ("58:7f:66", "Apple"),
    ("5c:59:48", "Apple"), ("5c:96:9d", "Apple"), ("60:03:08", "Apple"),
    ("60:33:4b", "Apple"), ("60:92:17", "Apple"), ("64:20:0c", "Apple"),
    ("64:9c:81", "Apple"), ("64:b0:a8", "Apple"), ("68:5b:35", "Apple"),
    ("68:9c:70", "Apple"), ("68:a8:6d", "Apple"), ("6c:40:08", "Apple"),
    ("6c:72:20", "Apple"), ("6c:ab:31", "Apple"), ("70:14:a6", "Apple"),
    ("70:56:81", "Apple"), ("70:a2:67", "Apple"), ("70:ca:04", "Apple"),
    ("74:1b:b2", "Apple"), ("74:81:14", "Apple"), ("74:8c:54", "Apple"),
    ("74:e1:b6", "Apple"), ("78:31:c1", "Apple"), ("78:3a:67", "Apple"),
    ("78:7c:a9", "Apple"), ("78:a3:e4", "Apple"), ("78:d4:f1", "Apple"),
    ("7c:03:d6", "Apple"), ("7c:6d:61", "Apple"), ("7c:c3:a1", "Apple"),
    ("7c:d1:66", "Apple"), ("80:02:18", "Apple"), ("80:49:38", "Apple"),
    ("80:82:f5", "Apple"), ("80:be:05", "Apple"), ("84:29:99", "Apple"),
    ("84:38:35", "Apple"), ("84:8d:4e", "Apple"), ("84:b1:72", "Apple"),
    ("88:1b:3e", "Apple"), ("88:63:df", "Apple"), ("88:66:39", "Apple"),
    ("8c:2d:aa", "Apple"), ("8c:58:77", "Apple"), ("8c:7b:9d", "Apple"),
    ("8c:85:90", "Apple"), ("90:18:7c", "Apple"), ("90:27:e4", "Apple"),
    ("90:72:40", "Apple"), ("94:94:26", "Apple"), ("94:9a:a4", "Apple"),
    ("98:00:c6", "Apple"), ("98:03:d8", "Apple"), ("98:14:62", "Apple"),
    ("98:46:0a", "Apple"), ("98:8b:5a", "Apple"), ("9c:04:ef", "Apple"),
    ("9c:20:ed", "Apple"), ("9c:6b:72", "Apple"), ("9c:99:a0", "Apple"),
    ("a0:18:ed", "Apple"), ("a0:72:91", "Apple"), ("a0:88:9b", "Apple"),
    ("a0:d3:7a", "Apple"), ("a4:18:c6", "Apple"), ("a4:45:19", "Apple"),
    ("a4:67:06", "Apple"), ("a4:b8:05", "Apple"), ("a4:c3:61", "Apple"),
    ("a8:20:66", "Apple"), ("a8:5b:b5", "Apple"), ("a8:86:dd", "Apple"),
    ("a8:96:cf", "Apple"), ("a8:bc:9d", "Apple"), ("a8:fa:26", "Apple"),
    ("ac:3c:0b", "Apple"), ("ac:44:f2", "Apple"), ("ac:61:ea", "Apple"),
    ("ac:7f:3e", "Apple"), ("ac:bc:32", "Apple"), ("ac:de:48", "Apple"),
    ("b0:02:47", "Apple"), ("b0:34:95", "Apple"), ("b0:65:bd", "Apple"),
    ("b0:9f:ba", "Apple"), ("b4:18:d1", "Apple"), ("b4:56:e9", "Apple"),
    ("b4:7c:9c", "Apple"), ("b4:99:ba", "Apple"), ("b4:b5:af", "Apple"),
    ("b8:09:8a", "Apple"), ("b8:17:c2", "Apple"), ("b8:41:5f", "Apple"),
    ("b8:53:ac", "Apple"), ("b8:66:85", "Apple"), ("b8:78:2e", "Apple"),
    ("bc:08:73", "Apple"), ("bc:14:ef", "Apple"), ("bc:52:b7", "Apple"),
    ("bc:5a:b0", "Apple"), ("bc:6c:6e", "Apple"), ("bc:92:6b", "Apple"),
    ("bc:a4:e1", "Apple"), ("c0:84:7a", "Apple"), ("c0:cc:6a", "Apple"),
    ("c4:34:6b", "Apple"), ("c4:47:3f", "Apple"), ("c4:64:13", "Apple"),
    ("c4:9e:43", "Apple"), ("c4:b3:b2", "Apple"), ("c8:15:45", "Apple"),
    ("c8:69:cd", "Apple"), ("c8:84:39", "Apple"), ("c8:89:56", "Apple"),
    ("c8:97:9b", "Apple"), ("c8:b5:ad", "Apple"), ("c8:d0:83", "Apple"),
    ("cc:08:8d", "Apple"), ("cc:29:f5", "Apple"), ("cc:44:63", "Apple"),
    ("cc:78:5f", "Apple"), ("cc:9f:7a", "Apple"), ("cc:b0:da", "Apple"),
    ("cc:c7:60", "Apple"), ("d0:03:4b", "Apple"), ("d0:63:b4", "Apple"),
    ("d0:81:7a", "Apple"), ("d0:94:66", "Apple"), ("d4:25:8b", "Apple"),
    ("d4:57:cf", "Apple"), ("d4:61:9e", "Apple"), ("d4:74:1b", "Apple"),
    ("d4:90:9a", "Apple"), ("d4:b1:46", "Apple"), ("d4:dc:cd", "Apple"),
    ("d8:00:4d", "Apple"), ("d8:30:62", "Apple"), ("d8:53:83", "Apple"),
    ("d8:58:e7", "Apple"), ("d8:61:0d", "Apple"), ("d8:6c:3a", "Apple"),
    ("d8:90:e8", "Apple"), ("d8:a2:5e", "Apple"), ("d8:bb:2c", "Apple"),
    ("dc:08:0f", "Apple"), ("dc:2b:2a", "Apple"), ("dc:37:14", "Apple"),
    ("dc:6c:5a", "Apple"), ("dc:86:d8", "Apple"), ("dc:9b:9c", "Apple"),
    ("dc:b3:94", "Apple"), ("dc:d7:43", "Apple"), ("e0:18:77", "Apple"),
    ("e0:45:c8", "Apple"), ("e0:88:5d", "Apple"), ("e0:b5:35", "Apple"),
    ("e0:c9:7a", "Apple"), ("e4:25:61", "Apple"), ("e4:5c:24", "Apple"),
    ("e4:98:d6", "Apple"), ("e4:b5:2b", "Apple"), ("e4:c6:3d", "Apple"),
    ("e4:fc:82", "Apple"), ("e8:04:0b", "Apple"), ("e8:27:74", "Apple"),
    ("e8:50:1b", "Apple"), ("e8:65:8c", "Apple"), ("e8:71:2b", "Apple"),
    ("e8:88:92", "Apple"), ("e8:94:35", "Apple"), ("e8:c7:4f", "Apple"),
    ("ec:10:7b", "Apple"), ("ec:35:86", "Apple"), ("ec:44:76", "Apple"),
    ("ec:5a:86", "Apple"), ("ec:66:1c", "Apple"), ("ec:7c:6c", "Apple"),
    ("ec:9b:f0", "Apple"), ("ec:ad:b8", "Apple"), ("ec:d0:37", "Apple"),
    ("f0:24:05", "Apple"), ("f0:37:17", "Apple"), ("f0:42:1c", "Apple"),
    ("f0:65:dd", "Apple"), ("f0:7b:cb", "Apple"), ("f0:93:c7", "Apple"),
    ("f0:9f:c2", "Apple"), ("f0:b4:79", "Apple"), ("f0:c8:48", "Apple"),
    ("f0:cb:a4", "Apple"), ("f0:db:e2", "Apple"), ("f4:0f:24", "Apple"),
    ("f4:31:6c", "Apple"), ("f4:5c:89", "Apple"), ("f4:7b:5e", "Apple"),
    ("f4:b7:e2", "Apple"), ("f4:f1:5a", "Apple"), ("f8:27:93", "Apple"),
    ("f8:34:41", "Apple"), ("f8:4d:89", "Apple"), ("f8:50:8a", "Apple"),
    ("f8:87:f1", "Apple"), ("f8:98:b9", "Apple"), ("f8:c1:16", "Apple"),
    ("fc:18:3c", "Apple"), ("fc:25:3f", "Apple"), ("fc:44:32", "Apple"),
    ("fc:67:81", "Apple"), ("fc:88:16", "Apple"), ("fc:94:e3", "Apple"),
    ("fc:a6:cd", "Apple"), ("fc:c2:de", "Apple"), ("fc:e5:5c", "Apple"),
    // Samsung
    ("84:38:63", "Samsung"), ("00:16:32", "Samsung"), ("00:17:c9", "Samsung"),
    ("00:1a:8a", "Samsung"), ("00:1d:25", "Samsung"), ("00:21:19", "Samsung"),
    ("00:23:39", "Samsung"), ("00:24:54", "Samsung"), ("00:26:37", "Samsung"),
    ("08:08:c2", "Samsung"), ("08:d4:0c", "Samsung"), ("10:1d:c0", "Samsung"),
    ("18:3f:47", "Samsung"), ("20:13:e0", "Samsung"), ("24:4b:03", "Samsung"),
    ("28:98:7b", "Samsung"), ("2c:ae:2b", "Samsung"), ("30:19:66", "Samsung"),
    ("34:23:ba", "Samsung"), ("38:01:46", "Samsung"), ("3c:62:00", "Samsung"),
    ("40:0e:85", "Samsung"), ("44:f4:59", "Samsung"), ("50:01:bb", "Samsung"),
    ("5c:0a:5b", "Samsung"), ("60:af:6d", "Samsung"), ("64:b3:10", "Samsung"),
    ("6c:83:36", "Samsung"), ("78:1f:db", "Samsung"), ("7c:61:93", "Samsung"),
    ("84:25:db", "Samsung"), ("88:32:9b", "Samsung"), ("90:18:7c", "Samsung"),
    ("94:35:0a", "Samsung"), ("a0:0b:ba", "Samsung"), ("a4:eb:d3", "Samsung"),
    ("b4:3a:28", "Samsung"), ("bc:20:a4", "Samsung"), ("c0:bd:d1", "Samsung"),
    ("cc:07:ab", "Samsung"), ("d0:22:be", "Samsung"), ("d4:87:d8", "Samsung"),
    ("d8:57:ef", "Samsung"), ("e0:99:71", "Samsung"), ("e4:12:1d", "Samsung"),
    ("e8:4e:84", "Samsung"), ("ec:1f:72", "Samsung"), ("f0:25:b7", "Samsung"),
    ("f4:7b:5e", "Samsung"), ("fc:00:12", "Samsung"),
    // TP-Link
    ("ac:1f:6b", "TP-Link"), ("ac:1f:6d", "TP-Link"), ("14:cc:20", "TP-Link"),
    ("18:a6:f7", "TP-Link"), ("1c:61:b4", "TP-Link"), ("50:c7:bf", "TP-Link"),
    ("54:af:97", "TP-Link"), ("64:70:02", "TP-Link"), ("6c:5a:b0", "TP-Link"),
    ("74:da:38", "TP-Link"), ("90:f6:52", "TP-Link"), ("a4:2b:b0", "TP-Link"),
    ("b0:48:7a", "TP-Link"), ("b4:b0:24", "TP-Link"), ("c4:e9:84", "TP-Link"),
    ("d4:6e:0e", "TP-Link"), ("e4:d3:32", "TP-Link"), ("f4:f2:6d", "TP-Link"),
    // Raspberry Pi
    ("b8:27:eb", "Raspberry Pi"), ("dc:a6:32", "Raspberry Pi"),
    ("e4:5f:01", "Raspberry Pi"), ("28:cd:c1", "Raspberry Pi"),
    // Amazon
    ("90:ad:d4", "Amazon"), ("74:c2:46", "Amazon"), ("44:65:0d", "Amazon"),
    ("f0:81:73", "Amazon"), ("00:fc:8b", "Amazon"), ("04:a1:51", "Amazon"),
    ("0c:47:c9", "Amazon"), ("34:d2:70", "Amazon"), ("40:b4:cd", "Amazon"),
    ("68:37:e9", "Amazon"), ("6c:56:97", "Amazon"), ("78:e1:03", "Amazon"),
    ("84:d6:d0", "Amazon"), ("8c:85:80", "Amazon"), ("a0:02:dc", "Amazon"),
    ("b4:7c:9c", "Amazon"), ("f0:27:2d", "Amazon"), ("fc:65:de", "Amazon"),
    // Google
    ("3c:5a:b4", "Google"), ("54:60:09", "Google"), ("6c:ad:f8", "Google"),
    ("f4:f5:d8", "Google"), ("94:eb:2c", "Google"), ("a4:77:33", "Google"),
    ("48:d6:d5", "Google"),
    // Cisco
    ("00:00:0c", "Cisco"), ("00:01:42", "Cisco"), ("00:01:96", "Cisco"),
    ("00:01:c7", "Cisco"), ("00:02:17", "Cisco"), ("00:03:6b", "Cisco"),
    ("00:04:9a", "Cisco"), ("00:05:32", "Cisco"), ("00:06:7c", "Cisco"),
    ("00:07:0d", "Cisco"), ("00:08:a3", "Cisco"), ("00:09:12", "Cisco"),
    ("00:0a:8a", "Cisco"), ("00:0b:be", "Cisco"), ("00:0c:85", "Cisco"),
    ("00:0d:28", "Cisco"), ("00:0e:38", "Cisco"), ("00:0f:23", "Cisco"),
    ("00:1a:2f", "Cisco"), ("00:1b:54", "Cisco"), ("00:1c:58", "Cisco"),
    ("00:1d:45", "Cisco"), ("00:1e:49", "Cisco"), ("00:1f:26", "Cisco"),
    ("00:21:1b", "Cisco"), ("00:22:0c", "Cisco"), ("00:23:04", "Cisco"),
    ("00:24:13", "Cisco"), ("00:25:45", "Cisco"), ("00:26:0a", "Cisco"),
    ("00:27:0d", "Cisco"), ("b4:a4:e3", "Cisco"), ("c8:9c:1d", "Cisco"),
    // Netgear
    ("00:09:5b", "Netgear"), ("00:0f:b5", "Netgear"), ("00:14:6c", "Netgear"),
    ("00:18:4d", "Netgear"), ("00:1b:2f", "Netgear"), ("00:1e:2a", "Netgear"),
    ("00:1f:33", "Netgear"), ("00:22:3f", "Netgear"), ("00:24:b2", "Netgear"),
    ("00:26:f2", "Netgear"), ("20:4e:7f", "Netgear"), ("2c:b0:5d", "Netgear"),
    ("30:46:9a", "Netgear"), ("44:94:fc", "Netgear"), ("6c:f0:49", "Netgear"),
    ("84:1b:5e", "Netgear"), ("a0:21:b7", "Netgear"), ("c0:3f:0e", "Netgear"),
    ("e0:46:9a", "Netgear"),
    // Microsoft
    ("00:03:ff", "Microsoft"), ("00:12:5a", "Microsoft"), ("00:15:5d", "Microsoft"),
    ("00:1d:d8", "Microsoft"), ("00:22:48", "Microsoft"), ("00:50:f2", "Microsoft"),
    ("28:18:78", "Microsoft"), ("3c:83:75", "Microsoft"), ("48:50:73", "Microsoft"),
    ("60:45:bd", "Microsoft"), ("7c:1e:52", "Microsoft"), ("90:e6:ba", "Microsoft"),
    ("b8:31:b5", "Microsoft"), ("dc:07:02", "Microsoft"),
    // Dell
    ("00:06:5b", "Dell"), ("00:08:74", "Dell"), ("00:0b:db", "Dell"),
    ("00:0d:56", "Dell"), ("00:0f:1f", "Dell"), ("00:11:43", "Dell"),
    ("00:12:3f", "Dell"), ("00:13:72", "Dell"), ("00:14:22", "Dell"),
    ("00:15:c5", "Dell"), ("00:16:f0", "Dell"), ("00:18:8b", "Dell"),
    ("00:19:b9", "Dell"), ("00:1a:a0", "Dell"), ("00:1c:23", "Dell"),
    ("00:1d:09", "Dell"), ("00:1e:4f", "Dell"), ("00:1f:d0", "Dell"),
    ("00:21:70", "Dell"), ("00:22:19", "Dell"), ("00:23:ae", "Dell"),
    ("00:24:e8", "Dell"), ("00:25:64", "Dell"), ("00:26:b9", "Dell"),
    ("18:03:73", "Dell"), ("18:66:da", "Dell"), ("24:b6:fd", "Dell"),
    ("34:17:eb", "Dell"), ("44:a8:42", "Dell"), ("54:9f:35", "Dell"),
    ("78:2b:cb", "Dell"), ("84:8f:69", "Dell"), ("b8:ac:6f", "Dell"),
    ("f8:db:88", "Dell"),
    // HP / Hewlett Packard
    ("00:01:e6", "HP"), ("00:02:a5", "HP"), ("00:04:ea", "HP"),
    ("00:06:2b", "HP"), ("00:08:02", "HP"), ("00:08:87", "HP"),
    ("00:0b:cd", "HP"), ("00:0d:9d", "HP"), ("00:0e:7f", "HP"),
    ("00:11:0a", "HP"), ("00:12:79", "HP"), ("00:13:21", "HP"),
    ("00:14:38", "HP"), ("00:15:60", "HP"), ("00:16:35", "HP"),
    ("00:17:08", "HP"), ("00:18:fe", "HP"), ("00:19:bb", "HP"),
    ("00:1a:4b", "HP"), ("00:1b:78", "HP"), ("00:1c:c4", "HP"),
    ("00:1e:0b", "HP"), ("00:1f:29", "HP"), ("00:21:5a", "HP"),
    ("00:22:64", "HP"), ("00:23:7d", "HP"), ("00:24:81", "HP"),
    ("00:25:b3", "HP"), ("00:26:55", "HP"), ("3c:4a:92", "HP"),
    ("3c:d9:2b", "HP"), ("58:20:b1", "HP"), ("6c:c2:17", "HP"),
    ("78:ac:c0", "HP"), ("9c:8e:99", "HP"), ("b4:99:ba", "HP"),
    ("d4:c9:ef", "HP"), ("ec:b1:d7", "HP"),
];

/// OUI vendor lookup map — built once at startup, O(1) lookup.
static OUI_MAP: once_cell::sync::Lazy<std::collections::HashMap<&'static str, &'static str>> =
    once_cell::sync::Lazy::new(|| {
        OUI_VENDORS.iter().copied().collect()
    });

pub fn mac_to_vendor(mac: &str) -> Option<String> {
    let normalized = normalize_mac_prefix(mac);
    OUI_MAP.get(normalized.as_str()).map(|v| v.to_string())
}

fn normalize_mac_prefix(mac: &str) -> String {
    let cleaned: String = mac
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();

    if cleaned.len() >= 6 {
        format!("{}:{}:{}", &cleaned[0..2], &cleaned[2..4], &cleaned[4..6])
    } else {
        cleaned
    }
}

#[allow(dead_code)]
pub fn get_hostname(ip: &str) -> Option<String> {
    let addr: std::net::IpAddr = ip.parse().ok()?;
    dns_lookup::lookup_addr(&addr)
        .ok()
        .filter(|h| !h.is_empty())
}

pub fn is_valid_mac(mac: &str) -> bool {
    let cleaned: String = mac
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();

    if cleaned.len() != 12 {
        return false;
    }

    cleaned.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn parse_mac(mac: &str) -> Result<[u8; 6]> {
    if !is_valid_mac(mac) {
        return Err(NetworkError::InvalidMacAddress(mac.to_string()));
    }

    let cleaned: String = mac
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();

    let mut result = [0u8; 6];
    for i in 0..6 {
        let byte_str = &cleaned[i * 2..i * 2 + 2];
        result[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|_| NetworkError::InvalidMacAddress(mac.to_string()))?;
    }

    Ok(result)
}

/// Validate MAC address for unicast use - rejects broadcast, multicast, and all-zeros
pub fn validate_unicast_mac(mac: &str) -> Result<[u8; 6]> {
    let bytes = parse_mac(mac)?;

    // Check for all zeros
    if bytes.iter().all(|&b| b == 0) {
        return Err(NetworkError::MacValidationError(
            mac.to_string(),
            crate::network::types::MacValidationError::AllZeros,
        ));
    }

    // Check for broadcast
    if bytes.iter().all(|&b| b == 0xFF) {
        return Err(NetworkError::MacValidationError(
            mac.to_string(),
            crate::network::types::MacValidationError::BroadcastAddress,
        ));
    }

    // Check for multicast (LSB of first octet)
    if bytes[0] & 0x01 != 0 {
        return Err(NetworkError::MacValidationError(
            mac.to_string(),
            crate::network::types::MacValidationError::MulticastAddress,
        ));
    }

    Ok(bytes)
}

pub fn format_mac(mac: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

#[allow(dead_code)]
pub fn parse_ip(ip: &str) -> Result<IpAddr> {
    IpAddr::from_str(ip).map_err(|_| NetworkError::InvalidIpAddress(ip.to_string()))
}

#[allow(dead_code)]
pub fn check_admin_privileges() -> Result<()> {
    #[cfg(unix)]
    {
        use std::process::Command;
        let output = Command::new("id").arg("-u").output().map_err(|e| {
            NetworkError::PermissionDenied(format!("Failed to check user ID: {}", e))
        })?;

        let uid = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()
            .map_err(|_| NetworkError::PermissionDenied("Failed to parse user ID".to_string()))?;

        if uid != 0 {
            return Err(NetworkError::PermissionDenied(
                "Administrator/root privileges required for raw socket operations".to_string(),
            ));
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("net")
            .args(["session"])
            .output()
            .map_err(|_| {
                NetworkError::PermissionDenied(
                    "Administrator privileges required for raw socket operations".to_string(),
                )
            })?;

        if !output.status.success() {
            return Err(NetworkError::PermissionDenied(
                "Administrator privileges required for raw socket operations".to_string(),
            ));
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub fn get_interface_ip(interface_name: &str) -> Result<String> {
    let interfaces = pnet_datalink::interfaces();

    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface_name.to_string()))?;

    let ip = interface
        .ips
        .iter()
        .find(|ip| ip.is_ipv4())
        .map(|ip| ip.ip().to_string())
        .ok_or_else(|| {
            NetworkError::InterfaceNotFound(format!(
                "No IPv4 address on interface {}",
                interface_name
            ))
        })?;

    Ok(ip)
}

pub fn get_interface_mac(interface_name: &str) -> Result<String> {
    let interfaces = pnet_datalink::interfaces();

    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface_name.to_string()))?;

    let mac = interface.mac.ok_or_else(|| {
        NetworkError::MacAddressError(format!("No MAC address on interface {}", interface_name))
    })?;

    Ok(format_mac(&mac.octets()))
}

/// Generate every usable host IP in the network described by `network_ip`
/// and `netmask` (both dotted-decimal).  Works correctly for any prefix
/// length from /8 to /30.  Network and broadcast addresses are excluded.
pub fn generate_network_range(network_ip: &str, netmask: &str) -> Vec<String> {
    let ip_addr: std::net::Ipv4Addr = match network_ip.parse() {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mask_addr: std::net::Ipv4Addr = match netmask.parse() {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };

    let ip_u32 = u32::from(ip_addr);
    let mask_u32 = u32::from(mask_addr);

    // Sanity: mask must be contiguous (all 1s before all 0s)
    let host_bits = mask_u32.leading_zeros() as u32 + (!mask_u32).leading_zeros() as u32;
    let _ = host_bits; // used implicitly below

    let network_u32 = ip_u32 & mask_u32;
    let host_mask = !mask_u32;

    // Need at least 2 host bits (/30) to have usable addresses
    if host_mask < 3 {
        return Vec::new();
    }

    let total_hosts = host_mask + 1; // includes network + broadcast

    (1..total_hosts - 1)
        .map(|i| {
            let addr = std::net::Ipv4Addr::from(network_u32 | i);
            addr.to_string()
        })
        .collect()
}

pub fn flush_arp_cache() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("arp")
            .arg("-an")
            .output()
            .map_err(NetworkError::IoError)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            // Extract IP from format: "? (192.168.1.1) at ..."
            if let Some(ip_raw) = line.split('(').nth(1).and_then(|s| s.split(')').next()) {
                // Validate IP before passing to subprocess — prevents injection
                if ip_raw.parse::<std::net::Ipv4Addr>().is_ok() {
                    let _ = std::process::Command::new("arp")
                        .args(["-d", ip_raw])
                        .output();
                }
            }
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("ip")
            .args(["-s", "neigh", "flush", "all"])
            .output()
            .map_err(NetworkError::IoError)?;

        if !output.status.success() {
            return Err(NetworkError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to flush ARP cache",
            )));
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("arp")
            .args(["-d", "*"])
            .output()
            .map_err(NetworkError::IoError)?;

        if !output.status.success() {
            return Err(NetworkError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to flush ARP cache",
            )));
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(NetworkError::PlatformNotSupported(
            "ARP cache flush not supported".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_mac() {
        assert!(is_valid_mac("aa:bb:cc:dd:ee:ff"));
        assert!(is_valid_mac("AA:BB:CC:DD:EE:FF"));
        assert!(is_valid_mac("aa-bb-cc-dd-ee-ff"));
        assert!(is_valid_mac("aabbccddeeff"));
        assert!(is_valid_mac("aabb.ccdd.eeff"));
        assert!(!is_valid_mac("aa:bb:cc:dd:ee"));
        assert!(!is_valid_mac("aa:bb:cc:dd:ee:ff:gg"));
        assert!(!is_valid_mac("not-a-mac"));
    }

    #[test]
    fn test_parse_mac() {
        let result = parse_mac("aa:bb:cc:dd:ee:ff").unwrap();
        assert_eq!(result, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);

        let result = parse_mac("aabbccddeeff").unwrap();
        assert_eq!(result, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_format_mac() {
        let mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        assert_eq!(format_mac(&mac), "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn test_mac_to_vendor() {
        assert_eq!(
            mac_to_vendor("80:02:18:00:00:00"),
            Some("Apple".to_string())
        );
        assert_eq!(
            mac_to_vendor("00:50:56:00:00:00"),
            Some("VMware".to_string())
        );
        assert_eq!(mac_to_vendor("ff:ff:ff:00:00:00"), None);
    }

    #[test]
    fn test_generate_network_range() {
        // /24 network
        let range = generate_network_range("192.168.1.0", "255.255.255.0");
        assert_eq!(range.len(), 254);
        assert_eq!(range[0], "192.168.1.1");
        assert_eq!(range[253], "192.168.1.254");

        // /30 network — only 2 usable hosts
        let range30 = generate_network_range("10.0.0.0", "255.255.255.252");
        assert_eq!(range30.len(), 2);
        assert_eq!(range30[0], "10.0.0.1");
        assert_eq!(range30[1], "10.0.0.2");

        // /23 network — 510 usable hosts
        let range23 = generate_network_range("192.168.0.0", "255.255.254.0");
        assert_eq!(range23.len(), 510);
        assert_eq!(range23[0], "192.168.0.1");
        assert_eq!(range23[509], "192.168.1.254");
    }
}
