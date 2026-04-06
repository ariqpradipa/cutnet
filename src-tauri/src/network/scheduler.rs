//! Scheduler engine for executing kill/restore schedules
//!
//! This module provides a tokio-based scheduler that periodically checks
//! for scheduled actions and executes them at the appropriate times.

#![allow(dead_code)]

use crate::network::schedules::{get_enabled_schedules, delete_schedule, get_next_execution};
use crate::network::types::{KillSchedule, ScheduleAction, ScheduleType};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

/// Scheduler state
pub struct Scheduler {
    running: Arc<RwLock<bool>>,
    killer_state: Arc<tokio::sync::Mutex<crate::ipc::state::Killer>>,
}

impl Scheduler {
    /// Create a new scheduler instance
    pub fn new(killer_state: Arc<tokio::sync::Mutex<crate::ipc::state::Killer>>) -> Self {
        Self {
            running: Arc::new(RwLock::new(false)),
            killer_state,
        }
    }

    /// Start the scheduler loop
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            log::info!("Scheduler already running");
            return;
        }
        *running = true;
        drop(running);

        log::info!("Starting scheduler loop");

        let running_flag = self.running.clone();
        let killer_state = self.killer_state.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60)); // Check every 60 seconds

            loop {
                ticker.tick().await;

                let is_running = *running_flag.read().await;
                if !is_running {
                    log::info!("Scheduler stopping");
                    break;
                }

                // Process schedules
                if let Err(e) = process_schedules(killer_state.clone()).await {
                    log::error!("Error processing schedules: {}", e);
                }
            }
        });
    }

    /// Stop the scheduler loop
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        log::info!("Scheduler stop requested");
    }

    /// Check if scheduler is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Process all enabled schedules and execute due actions
async fn process_schedules(
    killer_state: Arc<tokio::sync::Mutex<crate::ipc::state::Killer>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let schedules = get_enabled_schedules().await;
    let now = chrono::Local::now().timestamp() as u64;

    for schedule in schedules {
        if let Some(next_execution) = get_next_execution(&schedule.id).await {
            // Check if schedule is due (within the last minute)
            if next_execution <= now && now - next_execution < 60 {
                log::info!(
                    "Executing schedule {} for device {} ({})",
                    schedule.id,
                    schedule.device_ip,
                    schedule.device_mac
                );

                if let Err(e) = execute_schedule_action(&schedule, killer_state.clone()).await {
                    log::error!("Failed to execute schedule {}: {}", schedule.id, e);
                }

                // Delete one-time schedules after execution
                if let ScheduleType::OneTime { .. } = schedule.schedule_type {
                    if let Err(e) = delete_schedule(&schedule.id).await {
                        log::error!("Failed to delete one-time schedule {}: {}", schedule.id, e);
                    } else {
                        log::info!("Deleted one-time schedule {}", schedule.id);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Execute the action for a schedule
async fn execute_schedule_action(
    schedule: &KillSchedule,
    killer_state: Arc<tokio::sync::Mutex<crate::ipc::state::Killer>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match &schedule.action {
        ScheduleAction::Kill => {
            log::info!(
                "Schedule: Killing device {} ({})",
                schedule.device_ip,
                schedule.device_mac
            );

            let mut killer = killer_state.lock().await;
            killer
                .kill_device(schedule.device_ip.clone(), schedule.device_mac.clone())
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }
        ScheduleAction::Restore => {
            log::info!(
                "Schedule: Restoring device {} ({})",
                schedule.device_ip,
                schedule.device_mac
            );

            let mut killer = killer_state.lock().await;
            killer
                .unkill_device(schedule.device_ip.clone(), schedule.device_mac.clone())
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        }
        ScheduleAction::KillAndRestore { duration_minutes } => {
            log::info!(
                "Schedule: Kill and restore device {} ({}) for {} minutes",
                schedule.device_ip,
                schedule.device_mac,
                duration_minutes
            );

            // Kill the device
            {
                let mut killer = killer_state.lock().await;
                killer
                    .kill_device(schedule.device_ip.clone(), schedule.device_mac.clone())
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            }

            // Schedule restore after duration
            let duration = Duration::from_secs(*duration_minutes as u64 * 60);
            let killer_state_clone = killer_state.clone();
            let device_ip = schedule.device_ip.clone();
            let device_mac = schedule.device_mac.clone();
            let duration_mins = *duration_minutes;

            tokio::spawn(async move {
                tokio::time::sleep(duration).await;
                log::info!(
                    "Auto-restoring device {} ({}) after {} minutes",
                    device_ip,
                    device_mac,
                    duration_mins
                );

                let mut killer = killer_state_clone.lock().await;
                if let Err(e) = killer.unkill_device(device_ip.clone(), device_mac.clone()).await {
                    log::error!("Failed to auto-restore device {}: {}", device_ip, e);
                }
            });
        }
    }

    Ok(())
}
