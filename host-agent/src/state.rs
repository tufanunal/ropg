use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostSettings {
    pub relay_url: String,
    pub host_id: String,
    pub token: String,
    pub is_binary_mode: bool,
    pub quality: u8,
    pub grayscale: bool,
    pub interval_ms: u64,
    pub allow_remote_input: bool,
}

impl HostSettings {
    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write("rogp_settings.json", json);
        }
    }

    pub fn load_or_default(default_settings: HostSettings) -> Self {
        if let Ok(data) = fs::read_to_string("rogp_settings.json") {
            if let Ok(loaded) = serde_json::from_str::<HostSettings>(&data) {
                return loaded;
            }
        }
        default_settings
    }
}

pub struct LiveMetrics {
    pub is_connected: bool,
    pub last_seen_status: String,
    pub current_kb_per_sec: f32,
    pub dirty_cells_per_sec: u32,
    pub total_cells_updated: u64,
    pub total_bytes_sent: u64,
    pub total_frames_captured: u64,
    pub last_input_event: String,
    pub recent_dirty_cells_count: usize,
}

pub struct HostSharedState {
    pub is_running: AtomicBool,
    pub force_full_sync: AtomicBool,
    pub settings: Mutex<HostSettings>,
    pub metrics: Mutex<LiveMetrics>,
}

impl HostSharedState {
    pub fn new(
        relay_url: String,
        host_id: String,
        token: String,
        is_binary_mode: bool,
        quality: u8,
        grayscale: bool,
        interval_ms: u64,
    ) -> Arc<Self> {
        let initial = HostSettings {
            relay_url,
            host_id,
            token,
            is_binary_mode,
            quality,
            grayscale,
            interval_ms,
            allow_remote_input: true,
        };
        let active = HostSettings::load_or_default(initial);

        Arc::new(Self {
            is_running: AtomicBool::new(true),
            force_full_sync: AtomicBool::new(true),
            settings: Mutex::new(active),
            metrics: Mutex::new(LiveMetrics {
                is_connected: false,
                last_seen_status: "Initializing...".to_string(),
                current_kb_per_sec: 0.0,
                dirty_cells_per_sec: 0,
                total_cells_updated: 0,
                total_bytes_sent: 0,
                total_frames_captured: 0,
                last_input_event: "None".to_string(),
                recent_dirty_cells_count: 0,
            }),
        })
    }
}
