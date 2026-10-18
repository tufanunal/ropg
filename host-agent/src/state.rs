use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use std::fs;

fn default_grid_size() -> String {
    "40x40".to_string()
}

fn default_cell_size() -> usize {
    40
}

fn default_compression_method() -> i32 {
    5
}

fn default_dynamic_quality() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostSettings {
    pub relay_url: String,
    pub host_id: String,
    pub token: String,
    pub is_binary_mode: bool,
    pub quality: u8,
    pub grayscale: bool,
    pub interval_ms: u64,
    pub allow_input: bool,
    #[serde(default = "default_grid_size")]
    pub grid_size: String,
    #[serde(default = "default_cell_size")]
    pub cell_size: usize,
    #[serde(default = "default_compression_method")]
    pub compression_method: i32,
    #[serde(default = "default_dynamic_quality")]
    pub dynamic_quality: bool,
    #[serde(default)]
    pub show_token: bool,
}

impl HostSettings {
    pub fn get_grid_config(&self) -> crate::grid::GridConfig {
        crate::grid::GridConfig::from_preset(&self.grid_size)
    }

    pub fn grid_size_label(&self) -> &str {
        match self.grid_size.as_str() {
            "60x60" | "60" => "60x60 (32x18, 576 cells)",
            "120x120" | "120" => "120x120 (16x9, 144 cells)",
            "160x180" | "160" => "160x180 (12x6, 72 cells)",
            "240x270" | "240" => "240x270 (8x4, 32 cells)",
            "384x360" | "284x360" | "384" | "284" => "384x360 (5x3, 15 cells)",
            "640x540" | "640" => "640x540 (3x2, 6 cells)",
            _ => "40x40 (48x27, 1296 cells)",
        }
    }

    pub fn compression_label(&self) -> &str {
        match self.compression_method {
            4 => "Level 4 (Fastest, Low Compression)",
            6 => "Level 6 (Max Compression)",
            _ => "Level 5 (Normal Compression / Balanced)",
        }
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write("rogp_settings.json", json);
        }
    }

    pub fn load_or_default(default_settings: HostSettings) -> Self {
        if let Ok(data) = fs::read_to_string("rogp_settings.json") {
            if let Ok(mut loaded) = serde_json::from_str::<HostSettings>(&data) {
                // Ensure grid_size is synced if loaded from older config
                if loaded.grid_size.is_empty() {
                    loaded.grid_size = crate::grid::GridConfig::from_cell_size(loaded.cell_size).name.to_string();
                }
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
    pub peak_bandwidth_kbps: f32,
    pub dirty_cells_per_sec: u32,
    pub total_cells_updated: u64,
    pub total_bytes_sent: u64,
    pub total_frames_captured: u64,
    pub last_input_event: String,
    pub last_input_seq: u64,
    pub recent_dirty_cells_count: usize,
    pub rtt_latency_ms: u32,
    pub bandwidth_history: Vec<f32>,
    pub relay_test_status: Option<String>,
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
        use_saved_settings: bool,
    ) -> Arc<Self> {
        let initial = HostSettings {
            relay_url,
            host_id,
            token,
            is_binary_mode,
            quality,
            grayscale,
            interval_ms,
            allow_input: true,
            grid_size: "40x40".to_string(),
            cell_size: 40,
            compression_method: 5,
            dynamic_quality: true,
            show_token: false,
        };
        let active = if use_saved_settings {
            HostSettings::load_or_default(initial)
        } else {
            initial
        };

        Arc::new(Self {
            is_running: AtomicBool::new(true),
            force_full_sync: AtomicBool::new(true),
            settings: Mutex::new(active),
            metrics: Mutex::new(LiveMetrics {
                is_connected: false,
                last_seen_status: "Initializing...".to_string(),
                current_kb_per_sec: 0.0,
                peak_bandwidth_kbps: 0.0,
                dirty_cells_per_sec: 0,
                total_cells_updated: 0,
                total_bytes_sent: 0,
                total_frames_captured: 0,
                last_input_event: "None".to_string(),
                last_input_seq: 0,
                recent_dirty_cells_count: 0,
                rtt_latency_ms: 14,
                bandwidth_history: vec![0.0; 60],
                relay_test_status: None,
            }),
        })
    }
}
