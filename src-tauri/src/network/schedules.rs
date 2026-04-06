//! Schedule persistence and CRUD operations
//!
//! This module manages schedule persistence to disk and provides
//! thread-safe CRUD operations for managing device schedules.

use crate::network::types::{DayOfWeek, KillSchedule, ScheduleAction, ScheduleType};
use crate::network::NetworkError;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Data structure for persisting schedules
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SchedulesData {
    schedules: HashMap<String, KillSchedule>,
}

/// Global schedules manager instance
static SCHEDULES: Lazy<Arc<RwLock<SchedulesManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(SchedulesManager::new()))
});

/// Manager for schedule persistence and CRUD operations
pub struct SchedulesManager {
    schedules: HashMap<String, KillSchedule>,
    config_path: PathBuf,
}

impl SchedulesManager {
    /// Create a new SchedulesManager with default config path
    fn new() -> Self {
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cutnet");

        fs::create_dir_all(&config_path).ok();
        let config_path = config_path.join("schedules.json");

        let mut manager = Self {
            schedules: HashMap::new(),
            config_path,
        };

        manager.load();
        manager
    }

    /// Load schedules from disk
    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            if let Ok(data) = serde_json::from_str::<SchedulesData>(&content) {
                self.schedules = data.schedules;
            }
        }
    }

    /// Save schedules to disk
    fn save(&self) -> Result<(), NetworkError> {
        let data = SchedulesData {
            schedules: self.schedules.clone(),
        };
        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| NetworkError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        fs::write(&self.config_path, content)
            .map_err(|e| NetworkError::IoError(e))?;
        Ok(())
    }

    /// Create a new schedule
    pub async fn create_schedule(
        &mut self,
        device_mac: String,
        device_ip: String,
        action: ScheduleAction,
        schedule_type: ScheduleType,
    ) -> Result<String, NetworkError> {
        let id = Uuid::new_v4().to_string();
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Get current timezone offset
        let local = chrono::Local::now();
        let timezone_offset = local.offset().local_minus_utc() / 60; // Convert to minutes

        let schedule = KillSchedule {
            id: id.clone(),
            device_mac,
            device_ip,
            action,
            schedule_type,
            enabled: true,
            created_at,
            timezone_offset,
        };

        self.schedules.insert(id.clone(), schedule);
        self.save()?;
        Ok(id)
    }

    /// Get a schedule by ID
    pub async fn get_schedule(&self, id: &str) -> Option<KillSchedule> {
        self.schedules.get(id).cloned()
    }

    /// Get all schedules
    pub async fn get_all_schedules(&self) -> Vec<KillSchedule> {
        self.schedules.values().cloned().collect()
    }

    /// Get schedules for a specific device
    pub async fn get_device_schedules(&self, device_mac: &str) -> Vec<KillSchedule> {
        self.schedules
            .values()
            .filter(|s| s.device_mac == device_mac)
            .cloned()
            .collect()
    }

    /// Update a schedule
    pub async fn update_schedule(
        &mut self,
        id: &str,
        updates: ScheduleUpdate,
    ) -> Result<(), NetworkError> {
        if let Some(schedule) = self.schedules.get_mut(id) {
            if let Some(action) = updates.action {
                schedule.action = action;
            }
            if let Some(schedule_type) = updates.schedule_type {
                schedule.schedule_type = schedule_type;
            }
            if let Some(enabled) = updates.enabled {
                schedule.enabled = enabled;
            }
            self.save()?;
            Ok(())
        } else {
            Err(NetworkError::PoisoningError(format!("Schedule {} not found", id)))
        }
    }

    /// Delete a schedule
    pub async fn delete_schedule(&mut self, id: &str) -> Result<bool, NetworkError> {
        let removed = self.schedules.remove(id).is_some();
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// Toggle schedule enabled/disabled
    pub async fn toggle_schedule(&mut self, id: &str, enabled: bool) -> Result<(), NetworkError> {
        if let Some(schedule) = self.schedules.get_mut(id) {
            schedule.enabled = enabled;
            self.save()?;
            Ok(())
        } else {
            Err(NetworkError::PoisoningError(format!("Schedule {} not found", id)))
        }
    }

    /// Get next execution timestamp for a schedule
    pub async fn get_next_execution(&self, id: &str) -> Option<u64> {
        let schedule = self.schedules.get(id)?;
        if !schedule.enabled {
            return None;
        }
        calculate_next_execution(schedule)
    }
}

/// Updates that can be applied to a schedule
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScheduleUpdate {
    pub action: Option<ScheduleAction>,
    pub schedule_type: Option<ScheduleType>,
    pub enabled: Option<bool>,
}

/// Calculate the next execution time for a schedule
fn calculate_next_execution(schedule: &KillSchedule) -> Option<u64> {
    let now = chrono::Local::now();
    let now_timestamp = now.timestamp() as u64;

    match &schedule.schedule_type {
        ScheduleType::OneTime { execute_at } => {
            if *execute_at > now_timestamp {
                Some(*execute_at)
            } else {
                None
            }
        }
        ScheduleType::Daily { time } => {
            let today = now.date_naive();
            let scheduled_time = chrono::NaiveTime::from_hms_opt(time.hour as u32, time.minute as u32, 0)?;
            let scheduled_datetime = today.and_time(scheduled_time);
            
            // Convert to local datetime
            let scheduled_local = chrono::Local::now()
                .with_time(scheduled_time)
                .single()?;

            if scheduled_local > now {
                Some(scheduled_local.timestamp() as u64)
            } else {
                // Schedule for tomorrow
                Some((scheduled_local + chrono::Duration::days(1)).timestamp() as u64)
            }
        }
        ScheduleType::Weekly { days, time } => {
            if days.is_empty() {
                return None;
            }

            let scheduled_time = chrono::NaiveTime::from_hms_opt(time.hour as u32, time.minute as u32, 0)?;
            let current_weekday = now.weekday();
            let current_day_of_week = DayOfWeek::from_chrono(current_weekday);

            // Sort days
            let sorted_days: Vec<DayOfWeek> = {
                let mut d = days.clone();
                d.sort_by(|a, b| {
                    let a_num = day_to_number(a);
                    let b_num = day_to_number(b);
                    a_num.cmp(&b_num)
                });
                d
            };

            // Find next occurrence
            for day in &sorted_days {
                let day_num = day_to_number(day);
                let current_num = day_to_number(&current_day_of_week);

                if day_num > current_num {
                    // This day is later in the week
                    let days_ahead = day_num - current_num;
                    let target = now + chrono::Duration::days(days_ahead as i64);
                    let target_local = target
                        .with_time(scheduled_time)
                        .single()?;
                    return Some(target_local.timestamp() as u64);
                } else if day_num == current_num {
                    // Same day, check time
                    let today_scheduled = now.with_time(scheduled_time).single()?;
                    if today_scheduled > now {
                        return Some(today_scheduled.timestamp() as u64);
                    }
                }
            }

            // Wrap around to next week
            let first_day = sorted_days.first()?;
            let days_until = 7 - day_to_number(&current_day_of_week) + day_to_number(first_day);
            let target = now + chrono::Duration::days(days_until as i64);
            let target_local = target.with_time(scheduled_time).single()?;
            Some(target_local.timestamp() as u64)
        }
    }
}

/// Convert DayOfWeek to number (Monday=0, Sunday=6)
fn day_to_number(day: &DayOfWeek) -> u8 {
    match day {
        DayOfWeek::Monday => 0,
        DayOfWeek::Tuesday => 1,
        DayOfWeek::Wednesday => 2,
        DayOfWeek::Thursday => 3,
        DayOfWeek::Friday => 4,
        DayOfWeek::Saturday => 5,
        DayOfWeek::Sunday => 6,
    }
}

// Public API functions

/// Create a new schedule
pub async fn create_schedule(
    device_mac: String,
    device_ip: String,
    action: ScheduleAction,
    schedule_type: ScheduleType,
) -> Result<String, NetworkError> {
    let mut manager = SCHEDULES.write().await;
    manager.create_schedule(device_mac, device_ip, action, schedule_type).await
}

/// Get a schedule by ID
pub async fn get_schedule(id: &str) -> Option<KillSchedule> {
    let manager = SCHEDULES.read().await;
    manager.get_schedule(id).await
}

/// Get all schedules
pub async fn get_all_schedules() -> Vec<KillSchedule> {
    let manager = SCHEDULES.read().await;
    manager.get_all_schedules().await
}

/// Get schedules for a specific device
pub async fn get_device_schedules(device_mac: &str) -> Vec<KillSchedule> {
    let manager = SCHEDULES.read().await;
    manager.get_device_schedules(device_mac).await
}

/// Update a schedule
pub async fn update_schedule(id: &str, updates: ScheduleUpdate) -> Result<(), NetworkError> {
    let mut manager = SCHEDULES.write().await;
    manager.update_schedule(id, updates).await
}

/// Delete a schedule
pub async fn delete_schedule(id: &str) -> Result<bool, NetworkError> {
    let mut manager = SCHEDULES.write().await;
    manager.delete_schedule(id).await
}

/// Toggle schedule enabled/disabled
pub async fn toggle_schedule(id: &str, enabled: bool) -> Result<(), NetworkError> {
    let mut manager = SCHEDULES.write().await;
    manager.toggle_schedule(id, enabled).await
}

/// Get next execution timestamp for a schedule
pub async fn get_next_execution(id: &str) -> Option<u64> {
    let manager = SCHEDULES.read().await;
    manager.get_next_execution(id).await
}

/// Get all enabled schedules
pub async fn get_enabled_schedules() -> Vec<KillSchedule> {
    let manager = SCHEDULES.read().await;
    let all = manager.get_all_schedules().await;
    all.into_iter().filter(|s| s.enabled).collect()
}
