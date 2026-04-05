pub mod defender;
pub mod mac_ops;
pub mod poisoner;
pub mod scanner;
pub mod types;
pub mod utils;
pub mod whitelist;

pub use types::*;
pub use utils::{
    check_admin_privileges, flush_arp_cache, format_mac, generate_network_range, get_hostname,
    get_interface_ip, get_interface_mac, is_valid_mac, mac_to_vendor, parse_ip, parse_mac,
};
pub use mac_ops::{
    clone_mac, get_mac_address, get_original_mac, set_mac_address,
};
pub use poisoner::{
    get_poisoning_state, poison_once, start_poisoning, stop_poisoning,
};
pub use scanner::{
    arp_scan, get_all_interfaces, get_current_interface, ping_scan,
};
pub use defender::{
    clear_defender_alerts, get_defender_alerts, is_defender_active, start_defender_monitoring,
    stop_defender_monitoring, DefenderAlertEvent, SpoofAlert,
};
pub use whitelist::{
    add_entry as whitelist_add_entry, get_entries as whitelist_get_entries,
    is_protected as whitelist_is_protected, is_whitelisted as whitelist_is_whitelisted,
    remove_entry as whitelist_remove_entry, set_protect_enabled as whitelist_set_protect,
    WhitelistEntry,
};
