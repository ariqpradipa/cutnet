use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use pnet_datalink::{self, Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet_packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet_packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet_packet::Packet;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinSet;

use crate::network::types::{Device, NetworkError, Result};
use crate::network::utils::{format_mac, generate_network_range, mac_to_vendor};

const ARP_TIMEOUT_MS: u64 = 2000;
#[allow(dead_code)]
const PING_TIMEOUT_SECS: u64 = 2;
const SCAN_CONCURRENCY: usize = 50;

fn get_network_info(interface: &pnet_datalink::NetworkInterface) -> Option<(std::net::Ipv4Addr, u8)> {
    let ip = interface.ips.iter().find(|ip| ip.is_ipv4())?;
    if let std::net::IpAddr::V4(addr) = ip.ip() {
        Some((addr, ip.prefix()))
    } else {
        None
    }
}

pub async fn arp_scan(interface_name: &str) -> Result<Vec<Device>> {
    let interfaces = pnet_datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface_name.to_string()))?;

    let my_mac = interface
        .mac
        .ok_or_else(|| NetworkError::MacAddressError("Interface has no MAC".to_string()))?;

    let (source_ip, prefix_len) = get_network_info(&interface)
        .ok_or_else(|| NetworkError::InterfaceNotFound("No IPv4 on interface".to_string()))?;

    let network_prefix = format!(
        "{}.{}.{}",
        source_ip.octets()[0],
        source_ip.octets()[1],
        source_ip.octets()[2]
    );

    let netmask = prefix_to_netmask(prefix_len);
    let ip_range = generate_network_range(&network_prefix, &netmask);

    let (mut tx, mut rx) = create_arp_channel(&interface)?;

    let discovered = Arc::new(Mutex::new(HashMap::<String, Device>::new()));

    // Synchronization: ensure receiver is ready before sending first request
    let (ready_tx, ready_rx) = oneshot::channel();
    let recv_discovered = discovered.clone();
    let recv_task = tokio::spawn(async move {
        // Signal ready before starting to listen
        let _ = ready_tx.send(());
        receive_arp_replies(&mut rx, &recv_discovered).await;
    });

    // Wait for receiver to be ready
    let _ = ready_rx.await;
    tokio::time::sleep(Duration::from_millis(10)).await; // Small buffer

    let send_task = tokio::spawn(async move {
        send_arp_requests(&mut tx, &interface, source_ip, my_mac.octets(), &ip_range).await;
    });

    let _ = tokio::time::timeout(
        Duration::from_millis(ARP_TIMEOUT_MS),
        async {
            let _ = tokio::join!(recv_task, send_task);
        }
    ).await;

    let devices = discovered.lock().await.values().cloned().collect();
    Ok(devices)
}

fn prefix_to_netmask(prefix_len: u8) -> String {
    let mask = if prefix_len == 0 {
        0u32
    } else {
        0xffffffffu32 << (32 - prefix_len)
    };
    
    format!(
        "{}.{}.{}.{}",
        (mask >> 24) & 0xff,
        (mask >> 16) & 0xff,
        (mask >> 8) & 0xff,
        mask & 0xff
    )
}

fn create_arp_channel(
    interface: &NetworkInterface,
) -> Result<(Box<dyn DataLinkSender + Send>, Box<dyn DataLinkReceiver + Send>)> {
    let config = Config {
        read_timeout: Some(Duration::from_millis(100)),
        ..Default::default()
    };

    match pnet_datalink::channel(interface, config) {
        Ok(Channel::Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => Err(NetworkError::RawSocketError(
            "Unsupported channel type".to_string(),
        )),
        Err(e) => Err(NetworkError::RawSocketError(format!(
            "Failed to create channel: {} (requires root/admin)",
            e
        ))),
    }
}

async fn receive_arp_replies(
    rx: &mut Box<dyn DataLinkReceiver + Send>,
    discovered: &Arc<Mutex<HashMap<String, Device>>>,
) {
    loop {
        match rx.next() {
            Ok(packet) => {
                if let Some(ethernet) = EthernetPacket::new(packet) {
                    if ethernet.get_ethertype() == EtherTypes::Arp {
                        if let Some(arp) = ArpPacket::new(ethernet.payload()) {
                            if arp.get_operation() == ArpOperations::Reply {
                                let sender_ip = arp.get_sender_proto_addr();
                                let sender_mac = arp.get_sender_hw_addr();

                                let ip_str = sender_ip.to_string();
                                let mac_str = format_mac(&sender_mac.octets());

                                let vendor = mac_to_vendor(&mac_str);

                                let device = Device::new(ip_str.clone(), mac_str.clone())
                                    .with_vendor(vendor.unwrap_or_default());

                                let mut devices = discovered.lock().await;
                                devices.insert(ip_str, device);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
}

async fn send_arp_requests(
    tx: &mut Box<dyn DataLinkSender + Send>,
    interface: &NetworkInterface,
    source_ip: std::net::Ipv4Addr,
    source_mac: [u8; 6],
    targets: &[String],
) {
    for target_ip_str in targets {
        if let Ok(target_ip) = target_ip_str.parse::<std::net::Ipv4Addr>() {
            let packet = build_arp_request(interface, source_ip, source_mac, target_ip);

            let result = tx.send_to(&packet, Some(interface.clone()));
            if result.is_none() {
                log::warn!("Failed to send ARP request to {}", target_ip);
            }

            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}

fn build_arp_request(
    _interface: &NetworkInterface,
    source_ip: std::net::Ipv4Addr,
    source_mac: [u8; 6],
    target_ip: std::net::Ipv4Addr,
) -> Vec<u8> {
    let mut ethernet_buffer = vec![0u8; 14 + 28];
    
    {
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();
        ethernet_packet.set_destination([0xff, 0xff, 0xff, 0xff, 0xff, 0xff].into());
        ethernet_packet.set_source(source_mac.into());
        ethernet_packet.set_ethertype(EtherTypes::Arp);
    }
    
    {
        let mut arp_packet = MutableArpPacket::new(&mut ethernet_buffer[14..]).unwrap();
        arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
        arp_packet.set_protocol_type(EtherTypes::Ipv4);
        arp_packet.set_hw_addr_len(6);
        arp_packet.set_proto_addr_len(4);
        arp_packet.set_operation(ArpOperations::Request);
        arp_packet.set_sender_hw_addr(source_mac.into());
        arp_packet.set_sender_proto_addr(source_ip);
        arp_packet.set_target_hw_addr([0x00, 0x00, 0x00, 0x00, 0x00, 0x00].into());
        arp_packet.set_target_proto_addr(target_ip);
    }

    ethernet_buffer
}

pub async fn ping_scan(interface_name: &str) -> Result<Vec<Device>> {
    let interfaces = pnet_datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface_name.to_string()))?;

    let (source_ip, prefix_len) = get_network_info(&interface)
        .ok_or_else(|| NetworkError::InterfaceNotFound("No IPv4 on interface".to_string()))?;

    let network_prefix = format!(
        "{}.{}.{}",
        source_ip.octets()[0],
        source_ip.octets()[1],
        source_ip.octets()[2]
    );

    let netmask = prefix_to_netmask(prefix_len);
    let ip_range = generate_network_range(&network_prefix, &netmask);

    let mut join_set = JoinSet::new();
    let mut responded_ips: Vec<String> = Vec::new();

    for ip in ip_range {
        let ip_clone = ip.clone();
        join_set.spawn(async move {
            if ping_host(&ip_clone).await {
                Some(ip_clone)
            } else {
                None
            }
        });

        if join_set.len() >= SCAN_CONCURRENCY {
            if let Some(Ok(Some(ip))) = join_set.join_next().await {
                responded_ips.push(ip);
            }
        }
    }

    while let Some(result) = join_set.join_next().await {
        if let Ok(Some(ip)) = result {
            responded_ips.push(ip);
        }
    }

    let arp_table = read_arp_table().await?;

    let devices: Vec<Device> = arp_table
        .into_iter()
        .map(|(ip, mac)| {
            let vendor = mac_to_vendor(&mac);
            Device::new(ip, mac).with_vendor(vendor.unwrap_or_default())
        })
        .collect();

    Ok(devices)
}

async fn ping_host(ip: &str) -> bool {
    #[cfg(target_os = "macos")]
    let output = Command::new("ping")
        .args(["-c", "1", "-W", "1", ip])
        .output();

    #[cfg(target_os = "linux")]
    let output = Command::new("ping")
        .args(["-c", "1", "-W", "1", ip])
        .output();

    #[cfg(target_os = "windows")]
    let output = Command::new("ping")
        .args(["-n", "1", "-w", "1000", ip])
        .output();

    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}

async fn read_arp_table() -> Result<HashMap<String, String>> {
    let output = Command::new("arp")
        .arg("-an")
        .output()
        .map_err(|e| NetworkError::ArpScanError(format!("Failed to run arp: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = HashMap::new();

    for line in stdout.lines() {
        if let Some((ip, mac)) = parse_arp_entry(line) {
            if mac != "00:00:00:00:00:00" && mac != "ff:ff:ff:ff:ff:ff" {
                entries.insert(ip, mac);
            }
        }
    }

    Ok(entries)
}

fn parse_arp_entry(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 4 {
        return None;
    }

    let ip_part = parts.get(1)?;
    let ip = ip_part
        .trim_matches(|c| c == '(' || c == ')')
        .to_string();

    let mac = parts
        .iter()
        .find(|&&p| p.contains(':') && p.len() == 17)
        .map(|&m| m.to_string())?;

    Some((ip, mac))
}

pub fn get_current_interface() -> Result<crate::network::types::NetworkInterface> {
    let interfaces = pnet_datalink::interfaces();

    let interface = interfaces
        .into_iter()
        .find(|iface| {
            iface.is_up()
                && !iface.is_loopback()
                && iface.ips.iter().any(|ip| ip.is_ipv4())
                && iface.mac.is_some()
        })
        .ok_or_else(|| NetworkError::InterfaceNotFound("No active network interface found".to_string()))?;

    to_network_interface(&interface)
}

#[allow(dead_code)]
pub fn get_all_interfaces() -> Result<Vec<crate::network::types::NetworkInterface>> {
    let interfaces = pnet_datalink::interfaces();
    interfaces
        .iter()
        .filter(|iface| iface.ips.iter().any(|ip| ip.is_ipv4()))
        .map(|iface| to_network_interface(iface))
        .collect()
}

fn to_network_interface(iface: &NetworkInterface) -> Result<crate::network::types::NetworkInterface> {
    let ip = iface.ips.iter().find(|ip| ip.is_ipv4()).ok_or_else(|| {
        NetworkError::MacAddressError("No IPv4 address found".to_string())
    })?;
    let mac = iface
        .mac
        .ok_or_else(|| NetworkError::MacAddressError("No MAC address".to_string()))?;
    
    let (broadcast, netmask, ip_addr) = {
        let addr = if let std::net::IpAddr::V4(addr) = ip.ip() {
            addr
        } else {
            return Err(NetworkError::InvalidIpAddress("Not IPv4".into()));
        };
        let prefix = ip.prefix();
        
        let mask = !((1u32 << (32 - prefix)) - 1);
        let netmask_octets = mask.to_ne_bytes();
        let netmask = std::net::Ipv4Addr::from(netmask_octets);
        
        let network_addr: [u8; 4] = addr.octets().iter()
            .zip(netmask_octets.iter())
            .map(|(&ip_byte, &mask_byte)| ip_byte & mask_byte)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let broadcast_octets: [u8; 4] = network_addr.iter()
            .zip(netmask_octets.iter())
            .map(|(&net_byte, &mask_byte)| net_byte | !mask_byte)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let broadcast = std::net::Ipv4Addr::from(broadcast_octets);
        
        (broadcast.to_string(), netmask.to_string(), addr)
    };

    Ok(crate::network::types::NetworkInterface::new(
        iface.name.clone(),
        ip_addr.to_string(),
        format_mac(&mac.octets()),
        broadcast,
        netmask,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arp_entry() {
        let entry = "? (192.168.1.1) at ab:cd:ef:12:34:56 on en0 ifscope [ethernet]";
        let result = parse_arp_entry(entry);
        assert_eq!(result, Some(("192.168.1.1".to_string(), "ab:cd:ef:12:34:56".to_string())));

        let invalid = "? (192.168.1.1) at (incomplete) on en0 ifscope [ethernet]";
        assert!(parse_arp_entry(invalid).is_none());
    }

    #[test]
    fn test_get_all_interfaces() {
        let result = get_all_interfaces();
        assert!(result.is_ok());
        let interfaces = result.unwrap();
        assert!(!interfaces.is_empty());
    }
}
