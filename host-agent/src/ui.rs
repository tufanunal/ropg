use eframe::egui;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::state::HostSharedState;

pub struct RogpHostApp {
    state: Arc<HostSharedState>,
}

impl RogpHostApp {
    pub fn new(state: Arc<HostSharedState>) -> Self {
        Self { state }
    }
}

impl eframe::App for RogpHostApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Request repaint for live telemetry updates
        ui.ctx().request_repaint_after(Duration::from_millis(150));

        let is_running = self.state.is_running.load(Ordering::Relaxed);

        let (_metrics_connected, metrics_status, kb_s, cells_s, total_mb, last_input) = {
            let m = self.state.metrics.lock().unwrap();
            (
                m.is_connected,
                m.last_seen_status.clone(),
                m.current_kb_per_sec,
                m.dirty_cells_per_sec,
                m.total_bytes_sent as f64 / (1024.0 * 1024.0),
                m.last_input_event.clone(),
            )
        };

        ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);

        // 1. Header with Badge & Debian 12 info
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(" ROGP ")
                    .background_color(egui::Color32::from_rgb(6, 182, 212))
                    .color(egui::Color32::WHITE)
                    .strong()
                    .size(16.0),
            );
            ui.heading(
                egui::RichText::new("Host Agent")
                    .color(egui::Color32::from_rgb(248, 250, 252))
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new("Debian 12 (X11 1920x1080)")
                        .color(egui::Color32::from_rgb(148, 163, 184))
                        .size(11.0),
                );
            });
        });

        ui.separator();

        // 2. Status Banner & Start/Pause Button
        let frame_color = if is_running {
            egui::Color32::from_rgb(15, 34, 45) // Dark Cyan
        } else {
            egui::Color32::from_rgb(45, 20, 25) // Dark Rose
        };

        egui::Frame::new()
            .fill(frame_color)
            .corner_radius(8.0)
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let status_dot = if is_running { "🟢" } else { "🔴" };
                    let status_text = if is_running {
                        "ACTIVE & STREAMING"
                    } else {
                        "PAUSED (STREAM STOPPED)"
                    };
                    ui.label(
                        egui::RichText::new(format!("{} {}", status_dot, status_text))
                            .strong()
                            .size(14.0)
                            .color(if is_running {
                                egui::Color32::from_rgb(52, 211, 153)
                            } else {
                                egui::Color32::from_rgb(251, 113, 133)
                            }),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_label = if is_running { "Pause Sharing" } else { "Resume Sharing" };
                        let btn = egui::Button::new(
                            egui::RichText::new(btn_label)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        )
                        .fill(if is_running {
                            egui::Color32::from_rgb(225, 29, 72)
                        } else {
                            egui::Color32::from_rgb(16, 185, 129)
                        });

                        if ui.add(btn).clicked() {
                            self.state.is_running.store(!is_running, Ordering::Relaxed);
                        }
                    });
                });
            });

        // 3. Live Telemetry Card
        egui::CollapsingHeader::new(egui::RichText::new("📊 Live Telemetry & Metrics").strong())
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("telemetry_grid").num_columns(4).spacing([20.0, 8.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("Bandwidth:").color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new(format!("{:.1} KB/s", kb_s)).strong().color(egui::Color32::from_rgb(56, 189, 248)));

                    ui.label(egui::RichText::new("Dirty Rate:").color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new(format!("{} cells/s", cells_s)).strong());
                    ui.end_row();

                    ui.label(egui::RichText::new("Total Sent:").color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new(format!("{:.2} MB", total_mb)).strong());

                    let current_cs = { self.state.settings.lock().unwrap().cell_size };
                    let grid_cfg = crate::grid::GridConfig::from_cell_size(current_cs);
                    ui.label(egui::RichText::new("Grid Slices:").color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new(format!("{}x{} ({} cells)", grid_cfg.cols, grid_cfg.rows, grid_cfg.total_cells)).strong());
                    ui.end_row();

                    ui.label(egui::RichText::new("Last Input:").color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new(&last_input).color(egui::Color32::from_rgb(250, 204, 21)));

                    ui.label(egui::RichText::new("Status:").color(egui::Color32::GRAY));
                    ui.label(egui::RichText::new(&metrics_status).size(11.0));
                    ui.end_row();
                });
            });

        ui.separator();

        // 4. Connection Configuration
        egui::CollapsingHeader::new(egui::RichText::new("🌐 Connection Configuration").strong())
            .default_open(true)
            .show(ui, |ui| {
                let mut s = self.state.settings.lock().unwrap();

                ui.horizontal(|ui| {
                    ui.label("Host Identifier:");
                    ui.text_edit_singleline(&mut s.host_id);
                    if ui.button("🎲 Random").clicked() {
                        s.host_id = format!("host-{}", (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() % 1000));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Relay Server URL:");
                    ui.text_edit_singleline(&mut s.relay_url);
                });

                ui.horizontal(|ui| {
                    ui.label("Auth Token:");
                    ui.text_edit_singleline(&mut s.token);
                });

                ui.horizontal(|ui| {
                    if ui.button("💾 Save Settings").clicked() {
                        s.save();
                    }
                });
            });

        ui.separator();

        // 5. Streaming & Encoding Controls
        egui::CollapsingHeader::new(egui::RichText::new("⚡ Screen & Encoding Settings").strong())
            .default_open(true)
            .show(ui, |ui| {
                let mut s = self.state.settings.lock().unwrap();

                ui.horizontal(|ui| {
                    ui.label("Grid Resolution:");
                    let prev_cs = s.cell_size;
                    ui.radio_value(&mut s.cell_size, 40, "40px (48x27)");
                    ui.radio_value(&mut s.cell_size, 60, "60px (32x18)");
                    ui.radio_value(&mut s.cell_size, 120, "120px (16x9)");
                    if s.cell_size != prev_cs {
                        self.state.force_full_sync.store(true, Ordering::Relaxed);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Payload Mode:");
                    ui.radio_value(&mut s.is_binary_mode, true, "Mode B (Binary)");
                    ui.radio_value(&mut s.is_binary_mode, false, "Mode A (JSON + Base64)");
                });

                ui.horizontal(|ui| {
                    ui.label("WebP Quality:");
                    ui.add(egui::Slider::new(&mut s.quality, 10..=100).suffix("%"));
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut s.grayscale, "Grayscale Mode (Maximum bandwidth saving)");
                });

                ui.horizontal(|ui| {
                    ui.label("Polling Interval:");
                    ui.add(egui::Slider::new(&mut s.interval_ms, 50..=1000).suffix(" ms"));
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut s.allow_input, "Allow Mouse & Keyboard Control");
                });

                ui.add_space(6.0);
                let total = crate::grid::GridConfig::from_cell_size(s.cell_size).total_cells;
                if ui.button(egui::RichText::new(format!("🔄 Force Full Screen Sync (All {} Cells)", total)).strong()).clicked() {
                    self.state.force_full_sync.store(true, Ordering::Relaxed);
                }
            });

        ui.separator();
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new("Rogp • HTTP Polling Engine • Strictly No WebSockets")
                    .size(10.0)
                    .color(egui::Color32::DARK_GRAY),
            );
        });
    }
}
