use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::network::NetworkError;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceNamesData {
    names: HashMap<String, String>,
}

static DEVICE_NAMES: Lazy<Arc<RwLock<DeviceNamesManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(DeviceNamesManager::new()))
});

pub struct DeviceNamesManager {
    names: HashMap<String, String>,
    config_path: PathBuf,
}

impl DeviceNamesManager {
    fn new() -> Self {
        let config_path = std::env::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cutnet");

        fs::create_dir_all(&config_path).ok();
        let config_path = config_path.join("device_names.json");

        let mut manager = Self {
            names: HashMap::new(),
            config_path,
        };

        manager.load();
        manager
    }

    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            if let Ok(data) = serde_json::from_str::<DeviceNamesData>(&content) {
                self.names = data.names;
            }
        }
    }

    fn save(&self) -> Result<(), NetworkError> {
        let data = DeviceNamesData {
            names: self.names.clone(),
        };
        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| NetworkError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        fs::write(&self.config_path, content)
            .map_err(|e| NetworkError::IoError(e))?;
        Ok(())
    }

    pub async fn set_custom_name(&mut self, ip: String, name: String) -> Result<(), NetworkError> {
        if name.is_empty() {
            self.names.remove(&ip);
        } else {
            self.names.insert(ip, name);
        }
        self.save()
    }

    pub async fn get_custom_name(&self, ip: &str) -> Option<String> {
        self.names.get(ip).cloned()
    }

    pub async fn get_all_names(&self) -> HashMap<String, String> {
        self.names.clone()
    }
}

pub async fn set_custom_name(ip: String, name: String) -> Result<(), NetworkError> {
    let mut mgr = DEVICE_NAMES.write().await;
    mgr.set_custom_name(ip, name).await
}

pub async fn get_custom_name(ip: &str) -> Option<String> {
    let mgr = DEVICE_NAMES.read().await;
    mgr.get_custom_name(ip).await
}

pub async fn get_all_names() -> HashMap<String, String> {
    let mgr = DEVICE_NAMES.read().await;
    mgr.get_all_names().await
}
