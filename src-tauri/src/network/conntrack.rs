//! Connection tracking for packet forwarding
//!
//! This module implements connection tracking for TCP and UDP flows,
//! including TCP state machine transitions and cleanup of stale connections.

#![allow(dead_code)]

use crate::network::types::{
    ConnectionInfo, NetworkError, Protocol, Result, TcpState,
};
use pnet_packet::tcp::TcpFlags;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Default timeout for idle connections
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(300);
/// Cleanup interval for stale connections
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);
/// Maximum connections to track per session
const MAX_CONNECTIONS: usize = 10000;

/// Global connection tracker state
static CONNECTION_TRACKER: once_cell::sync::Lazy<RwLock<ConnectionTracker>> =
    once_cell::sync::Lazy::new(|| RwLock::new(ConnectionTracker::new()));

/// Manages tracked connections for packet forwarding
#[derive(Debug)]
pub struct ConnectionTracker {
    connections: HashMap<ConnectionKey, TrackedConnection>,
    last_cleanup: Instant,
}

/// Unique key identifying a connection
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConnectionKey {
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    protocol: Protocol,
}

/// A tracked network connection with metadata
#[derive(Debug, Clone)]
struct TrackedConnection {
    info: ConnectionInfo,
    last_activity: Instant,
    state: TcpState,
}

impl ConnectionTracker {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
            last_cleanup: Instant::now(),
        }
    }

    async fn maybe_cleanup(&mut self) {
        if self.last_cleanup.elapsed() > CLEANUP_INTERVAL {
            let now = Instant::now();
            let before_count = self.connections.len();

            self.connections
                .retain(|_, conn| now.duration_since(conn.last_activity) < DEFAULT_IDLE_TIMEOUT);

            let after_count = self.connections.len();
            if before_count != after_count {
                log::debug!(
                    "Cleaned up {} stale connections, {} remaining",
                    before_count - after_count,
                    after_count
                );
            }

            self.last_cleanup = now;
        }
    }
}

/// Track or update a connection
pub async fn track_connection(
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    protocol: Protocol,
    bytes: u64,
    tcp_flags: Option<u8>,
) -> Result<ConnectionInfo> {
    let mut tracker = CONNECTION_TRACKER.write().await;

    tracker.maybe_cleanup().await;

    if tracker.connections.len() >= MAX_CONNECTIONS {
        return Err(NetworkError::ConnectionTrackError(
            "Maximum connection limit reached".to_string(),
        ));
    }

    let key = ConnectionKey {
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        protocol,
    };

    let reverse_key = ConnectionKey {
        src_ip: dst_ip,
        dst_ip: src_ip,
        src_port: dst_port,
        dst_port: src_port,
        protocol,
    };

    let now = Instant::now();

    let connection = if let Some(conn) = tracker.connections.get_mut(&key) {
        conn.last_activity = now;
        conn.info.bytes_sent += bytes;
        conn.info.packets_sent += 1;
        conn.info.last_activity = chrono::Utc::now();

        if let Some(flags) = tcp_flags {
            conn.state = update_tcp_state(conn.state, flags, false);
            conn.info.state = conn.state;
        }

        conn.clone()
    } else if let Some(conn) = tracker.connections.get_mut(&reverse_key) {
        conn.last_activity = now;
        conn.info.bytes_received += bytes;
        conn.info.packets_received += 1;
        conn.info.last_activity = chrono::Utc::now();

        if let Some(flags) = tcp_flags {
            conn.state = update_tcp_state(conn.state, flags, true);
            conn.info.state = conn.state;
        }

        conn.clone()
    } else {
        let initial_state = if let Some(flags) = tcp_flags {
            if flags & TcpFlags::SYN != 0 && flags & TcpFlags::ACK == 0 {
                TcpState::SynSent
            } else if flags & TcpFlags::SYN != 0 && flags & TcpFlags::ACK != 0 {
                TcpState::SynReceived
            } else {
                TcpState::Established
            }
        } else {
            TcpState::Established
        };

        let info = ConnectionInfo::new(
            src_ip.to_string(),
            dst_ip.to_string(),
            src_port,
            dst_port,
            protocol,
        );

        let tracked = TrackedConnection {
            info: info.clone(),
            last_activity: now,
            state: initial_state,
        };

        tracker.connections.insert(key, tracked.clone());
        tracked
    };

    Ok(connection.info)
}

/// Get all active connections
pub async fn get_active_connections() -> Vec<ConnectionInfo> {
    let tracker = CONNECTION_TRACKER.read().await;
    tracker
        .connections
        .values()
        .map(|c| c.info.clone())
        .collect()
}

/// Get connections for a specific IP pair
pub async fn get_connections_between(
    ip1: Ipv4Addr,
    ip2: Ipv4Addr,
) -> Vec<ConnectionInfo> {
    let tracker = CONNECTION_TRACKER.read().await;
    tracker
        .connections
        .values()
        .filter(|c| {
            let src = c.info.src_ip.parse::<Ipv4Addr>();
            let dst = c.info.dst_ip.parse::<Ipv4Addr>();

            match (src, dst) {
                (Ok(s), Ok(d)) => {
                    (s == ip1 && d == ip2) || (s == ip2 && d == ip1)
                }
                _ => false,
            }
        })
        .map(|c| c.info.clone())
        .collect()
}

/// Get connection count
pub async fn get_connection_count() -> usize {
    let tracker = CONNECTION_TRACKER.read().await;
    tracker.connections.len()
}

/// Remove a specific connection
pub async fn remove_connection(
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    protocol: Protocol,
) -> bool {
    let mut tracker = CONNECTION_TRACKER.write().await;

    let key = ConnectionKey {
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        protocol,
    };

    let reverse_key = ConnectionKey {
        src_ip: dst_ip,
        dst_ip: src_ip,
        src_port: dst_port,
        dst_port: src_port,
        protocol,
    };

    tracker.connections.remove(&key).is_some()
        || tracker.connections.remove(&reverse_key).is_some()
}

/// Clear all tracked connections
pub async fn clear_all_connections() {
    let mut tracker = CONNECTION_TRACKER.write().await;
    let count = tracker.connections.len();
    tracker.connections.clear();
    log::info!("Cleared {} tracked connections", count);
}

/// Update TCP state based on flags and direction
fn update_tcp_state(current: TcpState, flags: u8, is_reverse: bool) -> TcpState {
    let syn = flags & TcpFlags::SYN != 0;
    let ack = flags & TcpFlags::ACK != 0;
    let fin = flags & TcpFlags::FIN != 0;
    let rst = flags & TcpFlags::RST != 0;

    match current {
        TcpState::SynSent => {
            if is_reverse && syn && ack {
                TcpState::Established
            } else if rst {
                TcpState::Closed
            } else {
                current
            }
        }
        TcpState::SynReceived => {
            if !is_reverse && ack {
                TcpState::Established
            } else if rst {
                TcpState::Closed
            } else {
                current
            }
        }
        TcpState::Established => {
            if fin {
                if is_reverse {
                    TcpState::CloseWait
                } else {
                    TcpState::FinWait1
                }
            } else if rst {
                TcpState::Closed
            } else {
                current
            }
        }
        TcpState::FinWait1 => {
            if is_reverse && fin {
                TcpState::Closing
            } else if is_reverse && ack {
                TcpState::FinWait2
            } else if rst {
                TcpState::Closed
            } else {
                current
            }
        }
        TcpState::FinWait2 => {
            if is_reverse && fin {
                TcpState::TimeWait
            } else if rst {
                TcpState::Closed
            } else {
                current
            }
        }
        TcpState::CloseWait => {
            if !is_reverse && fin {
                TcpState::LastAck
            } else if rst {
                TcpState::Closed
            } else {
                current
            }
        }
        TcpState::Closing => {
            if ack {
                TcpState::TimeWait
            } else if rst {
                TcpState::Closed
            } else {
                current
            }
        }
        TcpState::LastAck => {
            if is_reverse && ack {
                TcpState::Closed
            } else if rst {
                TcpState::Closed
            } else {
                current
            }
        }
        TcpState::TimeWait => {
            TcpState::Closed
        }
        TcpState::Closed => TcpState::Closed,
    }
}

/// Get TCP state as string
pub fn tcp_state_to_string(state: TcpState) -> &'static str {
    match state {
        TcpState::SynSent => "SYN_SENT",
        TcpState::SynReceived => "SYN_RECEIVED",
        TcpState::Established => "ESTABLISHED",
        TcpState::FinWait1 => "FIN_WAIT_1",
        TcpState::FinWait2 => "FIN_WAIT_2",
        TcpState::CloseWait => "CLOSE_WAIT",
        TcpState::Closing => "CLOSING",
        TcpState::LastAck => "LAST_ACK",
        TcpState::TimeWait => "TIME_WAIT",
        TcpState::Closed => "CLOSED",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_state_transitions() {
        assert_eq!(
            update_tcp_state(TcpState::SynSent, TcpFlags::SYN | TcpFlags::ACK, true),
            TcpState::Established
        );

        assert_eq!(
            update_tcp_state(TcpState::Established, TcpFlags::FIN, false),
            TcpState::FinWait1
        );

        assert_eq!(
            update_tcp_state(TcpState::Established, TcpFlags::FIN, true),
            TcpState::CloseWait
        );

        assert_eq!(
            update_tcp_state(TcpState::FinWait1, TcpFlags::FIN | TcpFlags::ACK, true),
            TcpState::Closing
        );

        assert_eq!(
            update_tcp_state(TcpState::LastAck, TcpFlags::ACK, true),
            TcpState::Closed
        );

        assert_eq!(
            update_tcp_state(TcpState::Established, TcpFlags::RST, false),
            TcpState::Closed
        );
    }
}