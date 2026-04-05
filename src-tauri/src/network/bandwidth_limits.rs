//! Bandwidth limit persistence
//!
//! This module handles persistence of bandwidth limits to disk,
//! loading them on startup, and applying them to devices.

use super::bandwidth::get_bandwidth_controller;
use super::bandwidth::BandwidthLimit;
use std::path::PathBuf;
use tokio::fs;

/// Path to the bandwidth limits file
fn get_limits_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".cutnet");
    path.push("bandwidth_limits.json");
    path
}

/// Ensure the config directory exists
async fn ensure_config_dir() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".cutnet");
    fs::create_dir_all(&path).await?;
    Ok(())
}

/// Load bandwidth limits from disk
pub async fn load_limits() -> Result<Vec<BandwidthLimit>, Box<dyn std::error::Error + Send + Sync>> {
    let path = get_limits_path();
    
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(&path).await?;
    let limits: Vec<BandwidthLimit> = serde_json::from_str(&contents)?;
    
    Ok(limits)
}

/// Save bandwidth limits to disk
pub async fn save_limits(limits: &[BandwidthLimit]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ensure_config_dir().await?;
    
    let path = get_limits_path();
    let contents = serde_json::to_string_pretty(limits)?;
    fs::write(&path, contents).await?;
    
    log::info!("Saved {} bandwidth limits to {:?}", limits.len(), path);
    Ok(())
}

/// Apply saved limits to the bandwidth controller
pub async fn apply_saved_limits() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let limits = load_limits().await?;
    
    if limits.is_empty() {
        log::info!("No saved bandwidth limits to apply");
        return Ok(());
    }

    let controller = get_bandwidth_controller().await;
    
    if let Some(ctrl) = controller {
        for limit in limits {
            if limit.enabled {
                match ctrl.set_limit(
                    &limit.mac,
                    limit.download_limit_kbps,
                    limit.upload_limit_kbps,
                ).await {
                    Ok(()) => {
                        log::info!("Applied bandwidth limit for MAC {}", limit.mac);
                    }
                    Err(e) => {
                        log::error!("Failed to apply bandwidth limit for MAC {}: {}", limit.mac, e);
                    }
                }
            }
        }
    } else {
        log::warn!("Bandwidth controller not initialized, cannot apply saved limits");
    }
    
    Ok(())
}

/// Save current limits from the bandwidth controller
pub async fn persist_current_limits() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let controller = get_bandwidth_controller().await;
    
    if let Some(ctrl) = controller {
        let limits = ctrl.get_limits().await;
        save_limits(&limits).await?;
    }
    
    Ok(())
}

/// Add or update a limit and persist it
pub async fn add_limit_and_persist(
    mac: &str,
    download_kbps: Option<u32>,
    upload_kbps: Option<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let controller = get_bandwidth_controller().await;
    
    if let Some(ctrl) = controller {
        ctrl.set_limit(mac, download_kbps, upload_kbps).await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
        
        // Persist all limits
        let limits = ctrl.get_limits().await;
        save_limits(&limits).await?;
        
        log::info!("Added bandwidth limit for MAC {} and persisted", mac);
    } else {
        return Err("Bandwidth controller not initialized".into());
    }
    
    Ok(())
}

/// Remove a limit and persist the change
pub async fn remove_limit_and_persist(mac: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let controller = get_bandwidth_controller().await;
    
    if let Some(ctrl) = controller {
        ctrl.remove_limit(mac).await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
        
        // Persist remaining limits
        let limits = ctrl.get_limits().await;
        save_limits(&limits).await?;
        
        log::info!("Removed bandwidth limit for MAC {} and persisted", mac);
    } else {
        return Err("Bandwidth controller not initialized".into());
    }
    
    Ok(())
}

/// Get all persisted limits
pub async fn get_persisted_limits() -> Result<Vec<BandwidthLimit>, Box<dyn std::error::Error + Send + Sync>> {
    load_limits().await
}

/// Clear all limits and remove persistence
pub async fn clear_all_limits() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let controller = get_bandwidth_controller().await;
    
    if let Some(ctrl) = controller {
        ctrl.remove_all_limits().await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
    }
    
    // Remove the persistence file
    let path = get_limits_path();
    if path.exists() {
        fs::remove_file(&path).await?;
        log::info!("Removed bandwidth limits persistence file");
    }
    
    Ok(())
}
