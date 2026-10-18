use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::capture::ScreenCapture;
use crate::client::RelayClient;
use crate::encoder::CellEncoder;
use crate::grid::GridDiffTracker;
use crate::input::InputDispatcher;
use crate::packet::{build_binary_packet, build_json_packet, EncodedCell};
use crate::state::HostSharedState;
use rayon::prelude::*;

pub fn start_worker_thread(state: Arc<HostSharedState>) {
    std::thread::spawn(move || {
        let mut capture = ScreenCapture::new();
        let initial_preset = { state.settings.lock().unwrap().grid_size.clone() };
        let mut diff_tracker = GridDiffTracker::new_with_preset(&initial_preset);
        let mut input_dispatcher = InputDispatcher::new();

        let mut last_metric_time = Instant::now();
        let mut bytes_since_metric = 0usize;
        let mut cells_since_metric = 0usize;
        let mut consecutive_fast_frames = 0usize;
        let mut fast_motion_active = false;

        let mut current_client = {
            let s = state.settings.lock().unwrap();
            RelayClient::new(s.relay_url.clone(), s.host_id.clone(), s.token.clone())
        };
        let mut cached_url = { state.settings.lock().unwrap().relay_url.clone() };
        let mut cached_id = { state.settings.lock().unwrap().host_id.clone() };

        let mut cached_token = { state.settings.lock().unwrap().token.clone() };

        loop {
            // 0. Update RelayClient immediately if connection parameters were changed in GUI
            {
                let s = state.settings.lock().unwrap();
                if s.relay_url != cached_url || s.host_id != cached_id || s.token != cached_token {
                    println!("[Worker] Connection target changed: URL={}, HostID={}", s.relay_url, s.host_id);
                    current_client = RelayClient::new(s.relay_url.clone(), s.host_id.clone(), s.token.clone());
                    cached_url = s.relay_url.clone();
                    cached_id = s.host_id.clone();
                    cached_token = s.token.clone();
                    state.force_full_sync.store(true, Ordering::Relaxed);
                }
            }

            // Check if user paused sharing via UI
            if !state.is_running.load(Ordering::Relaxed) {
                if last_metric_time.elapsed() >= Duration::from_millis(1000) {
                    let mut m = state.metrics.lock().unwrap();
                    m.last_seen_status = "Broadcast Paused".to_string();
                    m.current_kb_per_sec = 0.0;
                    m.dirty_cells_per_sec = 0;
                    if m.bandwidth_history.len() >= 60 {
                        m.bandwidth_history.remove(0);
                    }
                    m.bandwidth_history.push(0.0);
                    last_metric_time = Instant::now();
                }

                // While paused, still poll Relay for Resume command & config
                if let Ok((sync_requested, viewer_config, _inputs)) = current_client.poll_inputs() {
                    if sync_requested {
                        state.force_full_sync.store(true, Ordering::Relaxed);
                    }
                    if let Some(cfg) = viewer_config {
                        let mut s = state.settings.lock().unwrap();
                        if let Some(running) = cfg.is_running {
                            state.is_running.store(running, Ordering::Relaxed);
                            println!("[Config] Viewer toggled broadcasting: {}", if running { "RESUMED" } else { "PAUSED" });
                        }
                        if let Some(mode) = cfg.mode {
                            s.is_binary_mode = mode.to_lowercase() == "binary";
                        }
                        if let Some(q) = cfg.quality {
                            s.quality = q;
                        }
                        if let Some(grid_val) = cfg.grid_size.as_ref() {
                            let new_cfg = crate::grid::GridConfig::from_preset(grid_val);
                            if s.grid_size != new_cfg.name {
                                s.grid_size = new_cfg.name.to_string();
                                s.cell_size = new_cfg.cell_width;
                                println!("[Config] Viewer switched grid to: {} ({}x{} cells)", new_cfg.name, new_cfg.cols, new_cfg.rows);
                                state.force_full_sync.store(true, Ordering::Relaxed);
                            }
                        } else if let Some(cs_val) = cfg.cell_size {
                            let parsed = match cs_val {
                                serde_json::Value::Number(n) => crate::grid::GridConfig::from_cell_size(n.as_u64().unwrap_or(40) as usize),
                                serde_json::Value::String(ref str_val) => crate::grid::GridConfig::from_preset(str_val),
                                _ => crate::grid::GridConfig::from_preset("40x40"),
                            };
                            if s.grid_size != parsed.name {
                                s.grid_size = parsed.name.to_string();
                                s.cell_size = parsed.cell_width;
                                println!("[Config] Viewer switched grid to: {} ({}x{} cells)", parsed.name, parsed.cols, parsed.rows);
                                state.force_full_sync.store(true, Ordering::Relaxed);
                            }
                        }
                        if let Some(comp) = cfg.compression_method {
                            if (4..=6).contains(&comp) {
                                s.compression_method = comp;
                                println!("[Config] Viewer set compression method to: {}", comp);
                            }
                        }
                        if let Some(dyn_q) = cfg.dynamic_quality {
                            s.dynamic_quality = dyn_q;
                            println!("[Config] Viewer set dynamic quality to: {}", dyn_q);
                        }
                        if let Some(g) = cfg.grayscale {
                            s.grayscale = g;
                        }
                    }
                }

                sleep(Duration::from_millis(200));
                continue;
            }

            // 1. Poll pending input commands & config from Viewer first
            match current_client.poll_inputs() {
                Ok((sync_requested, viewer_config, inputs)) => {
                    if sync_requested {
                        state.force_full_sync.store(true, Ordering::Relaxed);
                    }

                    // Apply configuration from Viewer if received
                    if let Some(cfg) = viewer_config {
                        let mut s = state.settings.lock().unwrap();
                        if let Some(running) = cfg.is_running {
                            state.is_running.store(running, Ordering::Relaxed);
                            println!("[Config] Viewer toggled broadcasting: {}", if running { "RESUMED" } else { "PAUSED" });
                        }
                        if let Some(mode) = cfg.mode {
                            let new_bin = mode.to_lowercase() == "binary";
                            s.is_binary_mode = new_bin;
                            println!("[Config] Viewer switched transport mode to: {}", if new_bin { "Mode B (Binary)" } else { "Mode A (JSON)" });
                        }
                        if let Some(q) = cfg.quality {
                            s.quality = q;
                            println!("[Config] Viewer updated WebP quality to: {}%", q);
                        }
                        if let Some(grid_val) = cfg.grid_size.as_ref() {
                            let new_cfg = crate::grid::GridConfig::from_preset(grid_val);
                            if s.grid_size != new_cfg.name {
                                s.grid_size = new_cfg.name.to_string();
                                s.cell_size = new_cfg.cell_width;
                                println!("[Config] Viewer switched grid to: {} ({}x{} cells)", new_cfg.name, new_cfg.cols, new_cfg.rows);
                                state.force_full_sync.store(true, Ordering::Relaxed);
                            }
                        } else if let Some(cs_val) = cfg.cell_size {
                            let parsed = match cs_val {
                                serde_json::Value::Number(n) => crate::grid::GridConfig::from_cell_size(n.as_u64().unwrap_or(40) as usize),
                                serde_json::Value::String(ref str_val) => crate::grid::GridConfig::from_preset(str_val),
                                _ => crate::grid::GridConfig::from_preset("40x40"),
                            };
                            if s.grid_size != parsed.name {
                                s.grid_size = parsed.name.to_string();
                                s.cell_size = parsed.cell_width;
                                println!("[Config] Viewer switched grid to: {} ({}x{} cells)", parsed.name, parsed.cols, parsed.rows);
                                state.force_full_sync.store(true, Ordering::Relaxed);
                            }
                        }
                        if let Some(comp) = cfg.compression_method {
                            if (4..=6).contains(&comp) {
                                s.compression_method = comp;
                                println!("[Config] Viewer set compression method to: {}", comp);
                            }
                        }
                        if let Some(dyn_q) = cfg.dynamic_quality {
                            s.dynamic_quality = dyn_q;
                            println!("[Config] Viewer set dynamic quality to: {}", dyn_q);
                        }
                        if let Some(g) = cfg.grayscale {
                            s.grayscale = g;
                            println!("[Config] Viewer set grayscale to: {}", g);
                        }
                        if let Some(interval) = cfg.interval_ms {
                            s.interval_ms = interval;
                        }
                        state.force_full_sync.store(true, Ordering::Relaxed);
                    }

                    if !inputs.is_empty() {
                        let allow_in = { state.settings.lock().unwrap().allow_input };
                        let mut m = state.metrics.lock().unwrap();
                        let last_cmd = inputs.last().unwrap();
                        m.last_input_event = last_cmd.event_type.clone();
                        if let Some(seq) = last_cmd.seq_id {
                            m.last_input_seq = seq;
                        }

                        if allow_in {
                            drop(m);
                            for cmd in inputs {
                                input_dispatcher.dispatch(&cmd);
                            }
                        }
                    }
                }
                Err(_) => {}
            }

            // 2. Read dynamic streaming settings after config application
            let (is_binary_mode, base_quality, grayscale, interval_ms, grid_cfg, compression_method, dynamic_quality) = {
                let s = state.settings.lock().unwrap();
                (
                    s.is_binary_mode,
                    s.quality,
                    s.grayscale,
                    s.interval_ms,
                    s.get_grid_config(),
                    s.compression_method,
                    s.dynamic_quality,
                )
            };

            // Sync diff tracker grid size if changed
            if diff_tracker.config != grid_cfg {
                diff_tracker.set_preset(grid_cfg.name);
            }

            let loop_start = Instant::now();
            let force_full = state.force_full_sync.swap(false, Ordering::Relaxed);

            // 3. Capture screen buffer (1920x1080)
            let frame_buffer = capture.capture_frame();

            // 4. Diff grid
            let (dirty_cells, is_full_sync) = diff_tracker.diff_frame(&frame_buffer, force_full);
            let dirty_count = dirty_cells.len();
            let total_cells = grid_cfg.total_cells;

            // Calculate dynamic quality adjustment for fast changing scenes
            let effective_quality = if dynamic_quality && !is_full_sync && total_cells > 0 {
                let dirty_ratio = dirty_count as f32 / total_cells as f32;
                if dirty_ratio >= 0.35 {
                    // Fast scene with massive change (video/fast scrolling/drag)
                    consecutive_fast_frames += 1;
                    fast_motion_active = true;
                    ((base_quality as f32) * 0.50).clamp(20.0, 95.0) as u8
                } else if dirty_ratio >= 0.15 {
                    // Moderate motion
                    consecutive_fast_frames += 1;
                    fast_motion_active = true;
                    ((base_quality as f32) * 0.70).clamp(30.0, 95.0) as u8
                } else {
                    // Scene has stabilized: if coming out of rapid motion, force full crisp refresh
                    if fast_motion_active && consecutive_fast_frames >= 2 {
                        state.force_full_sync.store(true, Ordering::Relaxed);
                    }
                    consecutive_fast_frames = 0;
                    fast_motion_active = false;
                    base_quality
                }
            } else {
                base_quality
            };

            if !dirty_cells.is_empty() {
                // 5. Deduplicate identical dirty cells within frame (Intra Block Copy / Hash Matching)
                // Encode unique cells in parallel, then reuse the payload for identical duplicates
                use std::collections::HashMap;

                let encoder = CellEncoder::new(effective_quality, grayscale, grid_cfg.cell_width, grid_cfg.cell_height, compression_method);

                let mut unique_map: HashMap<u64, &crate::grid::DirtyCell> = HashMap::with_capacity(dirty_cells.len());
                for dc in &dirty_cells {
                    unique_map.entry(dc.hash).or_insert(dc);
                }

                let unique_cells: Vec<&crate::grid::DirtyCell> = unique_map.into_values().collect();
                let encoded_map: HashMap<u64, Vec<u8>> = unique_cells
                    .par_iter()
                    .filter_map(|dc| {
                        encoder.encode_cell(&dc.rgba_data).ok().map(|webp_bytes| (dc.hash, webp_bytes))
                    })
                    .collect();

                let mut encoded_cells = Vec::with_capacity(dirty_cells.len());
                for dc in &dirty_cells {
                    if let Some(webp_bytes) = encoded_map.get(&dc.hash) {
                        encoded_cells.push(EncodedCell {
                            row: dc.row as u8,
                            col: dc.col as u8,
                            cell_id: dc.cell_id.clone(),
                            version: dc.version,
                            webp_data: webp_bytes.clone(),
                        });
                    }
                }

                // 6. Build packet
                let (payload, is_bin) = if is_binary_mode {
                    (build_binary_packet(is_full_sync, grayscale, &encoded_cells), true)
                } else {
                    (build_json_packet(&cached_id, is_full_sync, &encoded_cells).into_bytes(), false)
                };

                let byte_len = payload.len();
                bytes_since_metric += byte_len;
                cells_since_metric += dirty_count;

                // 7. Send to Relay with RTT latency timing
                let send_start = Instant::now();
                match current_client.send_screen(is_bin, &payload, grid_cfg.name, grid_cfg.cell_width, effective_quality, compression_method, dynamic_quality) {
                    Ok(_) => {
                        let rtt = send_start.elapsed().as_millis().clamp(1, 9999) as u32;
                        let mut m = state.metrics.lock().unwrap();
                        m.is_connected = true;
                        m.rtt_latency_ms = rtt;
                        m.last_seen_status = format!("Transmitted {} cells ({} bytes) [{}]", dirty_count, byte_len, if is_bin { "BINARY" } else { "JSON" });
                        m.total_cells_updated += dirty_count as u64;
                        m.total_bytes_sent += byte_len as u64;
                    }
                    Err(err) => {
                        let mut m = state.metrics.lock().unwrap();
                        m.is_connected = false;
                        m.last_seen_status = format!("Network Error: {}", err);
                    }
                }
            }

            // Update rolling metrics every 1000ms
            if last_metric_time.elapsed() >= Duration::from_millis(1000) {
                let elapsed_secs = last_metric_time.elapsed().as_secs_f32();
                let kb_s = (bytes_since_metric as f32 / 1024.0) / elapsed_secs;
                let mut m = state.metrics.lock().unwrap();
                m.current_kb_per_sec = kb_s;
                if kb_s > m.peak_bandwidth_kbps {
                    m.peak_bandwidth_kbps = kb_s;
                }
                if m.bandwidth_history.len() >= 60 {
                    m.bandwidth_history.remove(0);
                }
                m.bandwidth_history.push(kb_s);
                m.dirty_cells_per_sec = (cells_since_metric as f32 / elapsed_secs) as u32;
                m.total_frames_captured += 1;
                m.recent_dirty_cells_count = dirty_count;

                bytes_since_metric = 0;
                cells_since_metric = 0;
                last_metric_time = Instant::now();
            }

            // Sleep target interval
            let target_interval = Duration::from_millis(interval_ms);
            let elapsed = loop_start.elapsed();
            if elapsed < target_interval {
                sleep(target_interval - elapsed);
            }
        }
    });
}
