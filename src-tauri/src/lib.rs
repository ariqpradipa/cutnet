//! CutNet - Network manipulation tool

mod ipc;
mod network;

use ipc::commands::*;
use ipc::state::init_state;
use network::scheduler::Scheduler;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        let _ = crate::network::poison_state::recover_from_crash().await;
        let _ = crate::network::apply_saved_limits().await;
    });

    let (killer_state, scanner_state) = init_state();
    let scheduler = Arc::new(Scheduler::new(killer_state.clone()));

    let scheduler_clone = scheduler.clone();
    rt.spawn(async move {
        scheduler_clone.start().await;
    });

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(killer_state)
        .manage(scanner_state)
        .manage(scheduler)
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
            start_defender,
            stop_defender,
            get_defender_alerts,
            clear_defender_alerts,
            is_defender_active,
            add_whitelist_entry,
            remove_whitelist_entry,
            get_whitelist_entries,
            set_whitelist_protect,
            is_whitelisted,
            flush_arp_cache_cmd,
            get_history,
            clear_history,
            set_device_custom_name,
            get_custom_device_names,
            set_bandwidth_limit,
            remove_bandwidth_limit,
            get_bandwidth_limits,
            get_bandwidth_stats,
            create_schedule,
            get_all_schedules,
            get_device_schedules,
            update_schedule,
            delete_schedule,
            toggle_schedule,
            start_forwarding,
            stop_forwarding,
            is_forwarding_active,
            add_forwarding_rule,
            remove_forwarding_rule,
            get_forwarding_rules,
            get_forwarding_stats,
            get_active_forwarding_sessions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
