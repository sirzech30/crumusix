use cpal::traits::{HostTrait, DeviceTrait};
use std::sync::OnceLock;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, serde::Serialize)]
pub struct AudioDeviceDetails {
    pub name: String,
    pub is_default: bool,
}

pub struct AudioDeviceManager {
    cached_devices: Mutex<Vec<String>>,
    default_device_name: Mutex<Option<String>>,
}

impl AudioDeviceManager {
    pub fn global() -> &'static Self {
        static INSTANCE: OnceLock<AudioDeviceManager> = OnceLock::new();
        INSTANCE.get_or_init(|| Self {
            cached_devices: Mutex::new(Vec::new()),
            default_device_name: Mutex::new(None),
        })
    }

    pub fn enumerate_output_devices(&self) -> Vec<AudioDeviceDetails> {
        let host = cpal::default_host();
        let default_device = host.default_output_device().and_then(|d| d.name().ok());
        
        let mut devices = Vec::new();
        if let Ok(output_devices) = host.output_devices() {
            for device in output_devices {
                if let Ok(name) = device.name() {
                    let is_default = default_device.as_ref().map_or(false, |d_name| d_name == &name);
                    devices.push(AudioDeviceDetails {
                        name,
                        is_default,
                    });
                }
            }
        }
        devices
    }

    pub fn start_hotplug_monitor(&self, app_handle: AppHandle) {
        let manager = Self::global();
        
        // Populate initial cache
        let initial = manager.enumerate_output_devices();
        *manager.cached_devices.lock() = initial.iter().map(|d| d.name.clone()).collect();
        *manager.default_device_name.lock() = initial.iter().find(|d| d.is_default).map(|d| d.name.clone());

        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(1500));
            loop {
                interval.tick().await;
                
                let current_devices = manager.enumerate_output_devices();
                let current_names: Vec<String> = current_devices.iter().map(|d| d.name.clone()).collect();
                let current_default = current_devices.iter().find(|d| d.is_default).map(|d| d.name.clone());

                let mut cached_names = manager.cached_devices.lock();
                let mut cached_default = manager.default_device_name.lock();

                // 1. Detect device changes (Addition / Removal)
                let added: Vec<&String> = current_names.iter().filter(|n| !cached_names.contains(n)).collect();
                let removed: Vec<&String> = cached_names.iter().filter(|n| !current_names.contains(n)).collect();

                if !added.is_empty() || !removed.is_empty() {
                    crate::log_info!("[AudioDevice] Hardware change detected! Added: {:?}, Removed: {:?}", added, removed);
                    
                    // Push diagnostics / status updates down to Tauri frontend
                    let _ = app_handle.emit("audio-devices-changed", current_devices.clone());
                    
                    // If playing and active device was removed, trigger PlaybackManager rebind
                    if let Some(ref active_def) = *cached_default {
                        if removed.contains(&active_def) {
                            crate::log_warn!("[AudioDevice] Active default output device removed! Rebinding streams...");
                            // Triggers playback safety pauses and rebinds
                            let _ = app_handle.emit("audio-device-lost", active_def.clone());
                        }
                    }
                }

                // 2. Detect default device change (Hot-swap events)
                if current_default != *cached_default {
                    crate::log_info!("[AudioDevice] Default output device changed: from {:?} to {:?}", *cached_default, current_default);
                    let _ = app_handle.emit("audio-default-changed", current_default.clone());
                }

                *cached_names = current_names;
                *cached_default = current_default;
            }
        });
    }
}
