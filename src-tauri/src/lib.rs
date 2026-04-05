//! CutNet - Network manipulation tool
//!
//! This is the main Tauri application entry point.

mod ipc;
mod network;

use ipc::commands::*;
use ipc::state::{init_state, cleanup_all_state};
use tauri::Listener;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (killer_state, scanner_state) = init_state();

    let killer_for_cleanup = killer_state.clone();
    let scanner_for_cleanup = scanner_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(killer_state.clone())
        .manage(scanner_state.clone())
        .setup(move |app| {
            let killer = killer_for_cleanup.clone();
            let scanner = scanner_for_cleanup.clone();

            let handle = app.handle().clone();
            app.listen("shutdown", move |_event| {
                let killer = killer.clone();
                let scanner = scanner.clone();
                tokio::spawn(async move {
                    cleanup_all_state(&killer, &scanner).await;
                    drop(handle);
                });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_interfaces,
            start_arp_scan,
            start_ping_scan,
            stop_scan,
            kill_device,
            unkill_device,
            kill_all_devices,
            unkill_all_devices,
            get_mac_address,
            set_mac_address,
            clone_mac_address,
            check_admin_privileges,
            get_system_info,
            ipc::commands::start_defender,
            ipc::commands::stop_defender,
            ipc::commands::get_defender_alerts,
            ipc::commands::clear_defender_alerts,
            ipc::commands::is_defender_active,
            ipc::commands::add_whitelist_entry,
            ipc::commands::remove_whitelist_entry,
            ipc::commands::get_whitelist_entries,
            ipc::commands::set_whitelist_protect,
            ipc::commands::is_whitelisted,
            ipc::commands::flush_arp_cache_cmd,
            ipc::commands::get_history,
            ipc::commands::clear_history,
            ipc::commands::set_device_custom_name,
            ipc::commands::get_custom_device_names,
            ipc::commands::set_bandwidth_limit,
            ipc::commands::remove_bandwidth_limit,
            ipc::commands::get_bandwidth_limits,
            ipc::commands::get_bandwidth_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
