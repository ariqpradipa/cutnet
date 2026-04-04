//! Event types and broadcasting for Tauri IPC
//!
//! This module defines all events that can be emitted from the Rust backend
//! to the frontend, along with helper functions for broadcasting them.

use crate::network::Device;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

/// Event emitted when a new device is discovered during network scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFoundEvent {
    /// The discovered device information
    pub device: Device,
}

/// Event emitted when a device is no longer responding or has left the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLostEvent {
    /// The device that was lost
    pub device: Device,
}

/// Event emitted during network scanning to report progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgressEvent {
    /// Progress percentage (0-100)
    pub progress: u8,
    /// Number of devices found so far
    pub devices_found: u16,
}

/// Event emitted when scan is completed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCompletedEvent {
    /// Total devices found
    pub total_devices: u16,
    /// Whether the scan completed successfully or was cancelled
    pub success: bool,
}

/// Event emitted when a device is killed (internet access blocked)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceKilledEvent {
    /// IP address of the device
    pub ip: String,
    /// MAC address of the device
    pub mac: String,
}

/// Event emitted when a device's internet access is restored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRestoredEvent {
    /// IP address of the device
    pub ip: String,
    /// MAC address of the device
    pub mac: String,
}

/// Event emitted when MAC address changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacAddressChangedEvent {
    /// Interface name
    pub interface: String,
    /// New MAC address
    pub new_mac: String,
}

/// Event emitted on errors that should be shown to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    /// Error message
    pub message: String,
    /// Optional error code for categorization
    pub code: Option<String>,
}

/// Emit a device found event to all frontend windows
pub fn emit_device_found(app: &AppHandle, device: Device) {
    let event = DeviceFoundEvent { device };
    if let Err(e) = app.emit("device-found", event) {
        log::error!("Failed to emit device-found event: {}", e);
    } else {
        log::debug!("Emitted device-found event");
    }
}

/// Emit a device lost event to all frontend windows
pub fn emit_device_lost(app: &AppHandle, device: Device) {
    let event = DeviceLostEvent { device };
    if let Err(e) = app.emit("device-lost", event) {
        log::error!("Failed to emit device-lost event: {}", e);
    } else {
        log::debug!("Emitted device-lost event");
    }
}

/// Emit scan progress update to all frontend windows
pub fn emit_scan_progress(app: &AppHandle, progress: u8, count: u16) {
    let event = ScanProgressEvent {
        progress,
        devices_found: count,
    };
    if let Err(e) = app.emit("scan-progress", event) {
        log::error!("Failed to emit scan-progress event: {}", e);
    } else {
        log::debug!(
            "Emitted scan-progress event: {}% with {} devices",
            progress,
            count
        );
    }
}

/// Emit scan completed event to all frontend windows
pub fn emit_scan_completed(app: &AppHandle, total_devices: u16, success: bool) {
    let event = ScanCompletedEvent {
        total_devices,
        success,
    };
    if let Err(e) = app.emit("scan-completed", event) {
        log::error!("Failed to emit scan-completed event: {}", e);
    } else {
        log::info!(
            "Emitted scan-completed event: {} devices, success: {}",
            total_devices,
            success
        );
    }
}

/// Emit device killed event to all frontend windows
pub fn emit_device_killed(app: &AppHandle, ip: String, mac: String) {
    let event = DeviceKilledEvent { ip, mac };
    if let Err(e) = app.emit("device-killed", event) {
        log::error!("Failed to emit device-killed event: {}", e);
    } else {
        log::debug!("Emitted device-killed event");
    }
}

/// Emit device restored event to all frontend windows
pub fn emit_device_restored(app: &AppHandle, ip: String, mac: String) {
    let event = DeviceRestoredEvent { ip, mac };
    if let Err(e) = app.emit("device-restored", event) {
        log::error!("Failed to emit device-restored event: {}", e);
    } else {
        log::debug!("Emitted device-restored event");
    }
}

/// Emit MAC address changed event to all frontend windows
pub fn emit_mac_address_changed(app: &AppHandle, interface: String, new_mac: String) {
    let interface_clone = interface.clone();
    let event = MacAddressChangedEvent { interface, new_mac };
    if let Err(e) = app.emit("mac-address-changed", event) {
        log::error!("Failed to emit mac-address-changed event: {}", e);
    } else {
        log::info!(
            "Emitted mac-address-changed event for interface {}",
            interface_clone
        );
    }
}

/// Emit error event to all frontend windows
pub fn emit_error(app: &AppHandle, message: String, code: Option<String>) {
    let event = ErrorEvent { message, code };
    if let Err(e) = app.emit("error", event) {
        log::error!("Failed to emit error event: {}", e);
    } else {
        log::debug!("Emitted error event");
    }
}
