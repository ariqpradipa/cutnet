// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Check if the application has administrator/root privileges synchronously.
/// Returns true if privileges are available, false otherwise.
fn check_admin_privileges_sync() -> bool {
    #[cfg(target_os = "windows")]
    {
        // On Windows, check if running with admin privileges
        use std::process::Command;
        let output = Command::new("net").args(["session"]).output();

        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // On Unix-like systems, check if EUID is 0 (root)
        unsafe { libc::geteuid() == 0 }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

fn main() {
    // Check privileges BEFORE tauri::Builder runs
    if !check_admin_privileges_sync() {
        eprintln!("⚠️  Administrator privileges required for full functionality");
        eprintln!("   Run with sudo (Linux/macOS) or as Administrator (Windows)");
        eprintln!("   The application will run in limited mode without admin privileges.");
        std::env::set_var("CUTNET_LIMITED_MODE", "1");
    } else {
        println!("✓ Administrator privileges confirmed");
    }

    cutnet_lib::run()
}
