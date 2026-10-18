use eframe::egui::{Align, Button, Color32, Layout, RichText, Ui, vec2};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::theme::*;
use crate::state::HostSharedState;

pub fn render_top_bar(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.heading(
            RichText::new("ROGP")
                .color(COLOR_ACCENT_CYAN)
                .strong()
                .size(18.0),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new("Host Agent")
                .color(COLOR_TEXT_PRIMARY)
                .strong()
                .size(14.0),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("| Debian 12 (X11) - FHD Grid Engine (48x27)")
                .color(COLOR_TEXT_MUTED)
                .size(11.5),
        );
    });
}

fn render_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), eframe::egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.5, color);
}

pub fn render_hero_status(ui: &mut Ui, state: &Arc<HostSharedState>) {
    let is_running = state.is_running.load(Ordering::Relaxed);
    let (relay_url, grid_cfg) = {
        let s = state.settings.lock().unwrap();
        (s.relay_url.clone(), s.get_grid_config())
    };
    let (kb_s, cells_s, total_mb, rtt_ms) = {
        let m = state.metrics.lock().unwrap();
        (
            m.current_kb_per_sec,
            m.dirty_cells_per_sec,
            m.total_bytes_sent as f64 / (1024.0 * 1024.0),
            m.rtt_latency_ms,
        )
    };

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let status_color = if is_running {
                COLOR_ACCENT_GREEN
            } else {
                COLOR_ACCENT_RED
            };
            let status_text = if is_running {
                "ACTIVE & STREAMING"
            } else {
                "STREAMING PAUSED"
            };

            // Left Section: Status & Relay URL stacked in 2 lines (compact 260px)
            ui.allocate_ui(vec2(260.0, 36.0), |ui| {
                ui.horizontal(|ui| {
                    render_dot(ui, status_color);
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(status_text)
                            .color(status_color)
                            .strong()
                            .size(12.5),
                    );
                });
                ui.label(
                    RichText::new(format!("Relay: {} (Connected - {}ms)", relay_url, rtt_ms))
                        .color(COLOR_TEXT_MUTED)
                        .size(10.0),
                );
            });

            ui.add_space(10.0);

            // Middle Section: 4 Stat Pills
            render_stat_pill(ui, "UPLOADING", &format!("{:.1} KB/s", kb_s), COLOR_ACCENT_CYAN);
            ui.add_space(6.0);
            render_stat_pill(ui, "DIRTY RATE", &format!("{} cells/s", cells_s), COLOR_ACCENT_BLUE);
            ui.add_space(6.0);
            render_stat_pill(ui, "TOTAL TRANSFERRED", &format!("{:.2} MB", total_mb), COLOR_TEXT_PRIMARY);
            ui.add_space(6.0);
            render_stat_pill(
                ui,
                "GRID RESOLUTION",
                &format!("{}x{} ({} cells)", grid_cfg.cols, grid_cfg.rows, grid_cfg.total_cells),
                COLOR_TEXT_MUTED,
            );

            // Right Section: Pause / Resume Sharing action button (right aligned)
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn_text = if is_running {
                    "Pause Sharing"
                } else {
                    "Resume Sharing"
                };
                let btn_color = if is_running {
                    COLOR_ACCENT_RED
                } else {
                    COLOR_ACCENT_GREEN
                };

                let btn = Button::new(
                    RichText::new(btn_text)
                        .color(if is_running { Color32::WHITE } else { Color32::BLACK })
                        .strong()
                        .size(12.5),
                )
                .fill(btn_color)
                .min_size(vec2(140.0, 34.0));

                if ui.add(btn).clicked() {
                    state.is_running.store(!is_running, Ordering::Relaxed);
                }
            });
        });
    });
}

fn render_stat_pill(ui: &mut Ui, label: &str, value: &str, val_color: Color32) {
    pill_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).color(COLOR_TEXT_MUTED).size(9.5));
            ui.add_space(2.0);
            ui.label(
                RichText::new(value)
                    .color(val_color)
                    .strong()
                    .size(11.0),
            );
        });
    });
}
