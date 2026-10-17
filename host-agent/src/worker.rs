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

pub fn start_worker_thread(state: Arc<HostSharedState>) {
    std::thread::spawn(move || {
        let mut capture = ScreenCapture::new();
        let initial_cs = { state.settings.lock().unwrap().cell_size };
        let mut diff_tracker = GridDiffTracker::new(initial_cs);
        let mut input_dispatcher = InputDispatcher::new();

        let mut last_metric_time = Instant::now();
        let mut bytes_since_metric = 0usize;
        let mut cells_since_metric = 0usize;

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
                {
                    let mut m = state.metrics.lock().unwrap();
                    m.last_seen_status = "Broadcast Paused".to_string();
                    m.current_kb_per_sec = 0.0;
                    m.dirty_cells_per_sec = 0;
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
                        if let Some(cs) = cfg.cell_size {
                            if (cs == 40 || cs == 60 || cs == 120) && s.cell_size != cs {
                                s.cell_size = cs;
                                println!("[Config] Viewer switched grid size to: {}px", cs);
                                state.force_full_sync.store(true, Ordering::Relaxed);
                            }
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
                        m.last_input_event = format!("{} (seq: {:?})", last_cmd.event_type, last_cmd.seq_id);

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
            let (is_binary_mode, quality, grayscale, interval_ms, cell_size) = {
                let s = state.settings.lock().unwrap();
                (
                    s.is_binary_mode,
                    s.quality,
                    s.grayscale,
                    s.interval_ms,
                    s.cell_size,
                )
            };

            // Sync diff tracker grid size if changed
            if diff_tracker.config.cell_size != cell_size {
                diff_tracker.set_cell_size(cell_size);
            }

            let loop_start = Instant::now();
            let force_full = state.force_full_sync.swap(false, Ordering::Relaxed);

            // 3. Capture screen buffer (1920x1080)
            let frame_buffer = capture.capture_frame();

            // 4. Diff grid
            let (dirty_cells, is_full_sync) = diff_tracker.diff_frame(&frame_buffer, force_full);
            let dirty_count = dirty_cells.len();

            if !dirty_cells.is_empty() {
                // 5. WebP encode
                let encoder = CellEncoder::new(quality, grayscale, cell_size, cell_size);
                let mut encoded_cells = Vec::with_capacity(dirty_cells.len());

                for dc in &dirty_cells {
                    if let Ok(webp_bytes) = encoder.encode_cell(&dc.rgba_data) {
                        encoded_cells.push(EncodedCell {
                            row: dc.row as u8,
                            col: dc.col as u8,
                            cell_id: dc.cell_id.clone(),
                            version: dc.version,
                            webp_data: webp_bytes,
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

                // 7. Send to Relay
                match current_client.send_screen(is_bin, &payload, cell_size, quality) {
                    Ok(_) => {
                        let mut m = state.metrics.lock().unwrap();
                        m.is_connected = true;
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
                let mut m = state.metrics.lock().unwrap();
                m.current_kb_per_sec = (bytes_since_metric as f32 / 1024.0) / elapsed_secs;
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
