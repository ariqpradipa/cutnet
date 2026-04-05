use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use pnet_datalink::{self, Channel, Config};
use pnet_packet::arp::{ArpOperations, ArpPacket};
use pnet_packet::ethernet::EthernetPacket;
use pnet_packet::Packet;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpoofAlert {
    pub timestamp: u64,
    pub claimed_ip: String,
    pub legitimate_mac: String,
    pub attacker_mac: String,
    pub attacker_ip: Option<String>,
    pub alert_type: String,
}

static DEFENDER_STATE: Lazy<RwLock<DefenderState>> = Lazy::new(|| {
    RwLock::new(DefenderState {
        known_mappings: HashMap::new(),
        alerts: Vec::new(),
        is_active: false,
        packet_counts: HashMap::new(),
    })
});

static DEFENDER_HANDLE: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

struct DefenderState {
    known_mappings: HashMap<String, String>,
    alerts: Vec<SpoofAlert>,
    is_active: bool,
    packet_counts: HashMap<String, (u64, Instant)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefenderAlertEvent {
    pub timestamp: u64,
    pub claimed_ip: String,
    pub legitimate_mac: String,
    pub attacker_mac: String,
    pub alert_type: String,
}

pub fn start_defender_monitoring(interface_name: &str, app: &AppHandle) -> Result<(), crate::network::NetworkError> {
    let interface_name = interface_name.to_string();
    let app = app.clone();
    
    let handle = tokio::spawn(async move {
        defender_monitor_loop(interface_name, app).await;
    });
    
    let rt = tokio::runtime::Handle::current();
    rt.block_on(async {
        let mut state = DEFENDER_STATE.write().await;
        state.is_active = true;
        let mut handle_opt = DEFENDER_HANDLE.lock().await;
        *handle_opt = Some(handle);
    });
    
    Ok(())
}

async fn defender_monitor_loop(interface_name: String, app: AppHandle) {
    let interfaces = pnet_datalink::interfaces();
    let interface = match interfaces.into_iter().find(|iface| iface.name == interface_name) {
        Some(iface) => iface,
        None => {
            log::error!("Defender: Interface {} not found", interface_name);
            return;
        }
    };

    let config = Config {
        read_timeout: Some(Duration::from_millis(100)),
        ..Default::default()
    };

    let (mut tx, mut rx) = match pnet_datalink::channel(&interface, config) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => {
            log::error!("Defender: Unsupported channel type");
            return;
        }
        Err(e) => {
            log::error!("Defender: Failed to create channel: {}", e);
            return;
        }
    };

    // Suppress unused variable warning for tx - we only listen
    let _ = tx;

    let mut last_rate_reset = Instant::now();
    
    loop {
        {
            let state = DEFENDER_STATE.read().await;
            if !state.is_active {
                break;
            }
        }

        match rx.next() {
            Ok(packet) => {
                if let Some(ethernet) = EthernetPacket::new(packet) {
                    if ethernet.get_ethertype() == pnet_packet::ethernet::EtherTypes::Arp {
                        if let Some(arp) = ArpPacket::new(ethernet.payload()) {
                            if arp.get_operation() == ArpOperations::Reply {
                                let sender_ip = arp.get_sender_proto_addr().to_string();
                                let sender_mac: String = arp.get_sender_hw_addr().octets().iter()
                                    .map(|b| format!("{:02x}", b))
                                    .collect::<Vec<_>>()
                                    .join(":");

                                // Check for IP conflict
                                {
                                    let mut state = DEFENDER_STATE.write().await;
                                    if let Some(known_mac) = state.known_mappings.get(&sender_ip) {
                                        if known_mac != &sender_mac {
                                            let alert = SpoofAlert {
                                                timestamp: std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap_or_default()
                                                    .as_secs(),
                                                claimed_ip: sender_ip.clone(),
                                                legitimate_mac: known_mac.clone(),
                                                attacker_mac: sender_mac.clone(),
                                                attacker_ip: None,
                                                alert_type: "ip_conflict".to_string(),
                                            };
                                            state.alerts.push(alert.clone());
                                            
                                            // Keep max 100 alerts
                                            if state.alerts.len() > 100 {
                                                state.alerts.remove(0);
                                            }

                                            let _ = app.emit("arp-spoof-detected", DefenderAlertEvent {
                                                timestamp: alert.timestamp,
                                                claimed_ip: alert.claimed_ip,
                                                legitimate_mac: alert.legitimate_mac,
                                                attacker_mac: alert.attacker_mac,
                                                alert_type: alert.alert_type,
                                            });
                                        }
                                    } else {
                                        state.known_mappings.insert(sender_ip.clone(), sender_mac.clone());
                                    }

                                    // Track packet rate
                                    let now = Instant::now();
                                    if now.duration_since(last_rate_reset) > Duration::from_secs(1) {
                                        state.packet_counts.clear();
                                        last_rate_reset = now;
                                    }

                                    let entry = state.packet_counts.entry(sender_mac.clone()).or_insert((0, now));
                                    entry.0 += 1;

                                    if entry.0 > 10 {
                                        let alert = SpoofAlert {
                                            timestamp: std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default()
                                                .as_secs(),
                                            claimed_ip: sender_ip.clone(),
                                            legitimate_mac: sender_mac.clone(),
                                            attacker_mac: sender_mac.clone(),
                                            attacker_ip: None,
                                            alert_type: "high_rate".to_string(),
                                        };
                                        state.alerts.push(alert.clone());
                                        
                                        if state.alerts.len() > 100 {
                                            state.alerts.remove(0);
                                        }

                                        let _ = app.emit("arp-spoof-detected", DefenderAlertEvent {
                                            timestamp: alert.timestamp,
                                            claimed_ip: alert.claimed_ip,
                                            legitimate_mac: alert.legitimate_mac,
                                            attacker_mac: alert.attacker_mac,
                                            alert_type: alert.alert_type,
                                        });
                                    }
                                }
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

pub async fn stop_defender_monitoring() -> Result<(), crate::network::NetworkError> {
    let mut state = DEFENDER_STATE.write().await;
    state.is_active = false;
    
    let mut handle_opt = DEFENDER_HANDLE.lock().await;
    if let Some(handle) = handle_opt.take() {
        handle.abort();
    }
    
    Ok(())
}

pub async fn get_defender_alerts() -> Vec<SpoofAlert> {
    let state = DEFENDER_STATE.read().await;
    state.alerts.clone()
}

pub async fn clear_defender_alerts() {
    let mut state = DEFENDER_STATE.write().await;
    state.alerts.clear();
}

pub async fn is_defender_active() -> bool {
    let state = DEFENDER_STATE.read().await;
    state.is_active
}
