//! CutNet - Network manipulation tool
//!
//! This is the main Tauri application entry point.

mod ipc;
mod network;

use ipc::commands::*;
use ipc::state::{init_state, KillerState, ScannerState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize shared state
    let (killer_state, scanner_state) = init_state();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(killer_state)
        .manage(scanner_state)
        .invoke_handler(tauri::generate_handler![
            // Network Discovery
            get_interfaces,
            start_arp_scan,
            start_ping_scan,
            stop_scan,
            // Device Control
            kill_device,
            unkill_device,
            kill_all_devices,
            unkill_all_devices,
            // MAC Operations
            get_mac_address,
            set_mac_address,
            clone_mac_address,
            // System Information
            check_admin_privileges,
            get_system_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
