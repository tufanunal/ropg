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
                .size(20.0),
        );
        ui.label(
            RichText::new("Host Agent v1.2.0")
                .color(COLOR_TEXT_PRIMARY)
                .size(14.0),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new("Debian 12 (X11) • FHD Grid Engine (48x27)")
                .color(COLOR_TEXT_MUTED)
                .size(12.0),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new("● Live Node")
                    .color(COLOR_ACCENT_GREEN)
                    .size(11.0),
            );
        });
    });
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
        ui.horizontal(|ui| {
            // Status badge
            let status_color = if is_running {
                COLOR_ACCENT_GREEN
            } else {
                COLOR_ACCENT_RED
            };
            let status_text = if is_running {
                "🟢 ACTIVE & STREAMING"
            } else {
                "🔴 STREAMING PAUSED"
            };

            ui.label(
                RichText::new(status_text)
                    .color(status_color)
                    .strong()
                    .size(14.0),
            );

            ui.add_space(12.0);

            // Connected Relay tag
            ui.label(
                RichText::new(format!(
                    "Relay: {} (Connected - {}ms)",
                    relay_url, rtt_ms
                ))
                .color(COLOR_TEXT_MUTED)
                .size(12.0),
            );

            // Right-aligned Pause / Resume Sharing action button
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn_text = if is_running {
                    "🔴 Pause Sharing"
                } else {
                    "🟢 Resume Sharing"
                };
                let btn_color = if is_running {
                    COLOR_ACCENT_RED
                } else {
                    COLOR_ACCENT_GREEN
                };

                let btn = Button::new(
                    RichText::new(btn_text)
                        .color(Color32::WHITE)
                        .strong()
                        .size(13.0),
                )
                .fill(btn_color)
                .min_size(vec2(140.0, 36.0));

                if ui.add(btn).clicked() {
                    state.is_running.store(!is_running, Ordering::Relaxed);
                }
            });
        });

        ui.add_space(8.0);

        // Quick Stats Pills row
        ui.horizontal(|ui| {
            render_stat_pill(
                ui,
                "UPLOADING",
                &format!("{:.1} KB/s", kb_s),
                COLOR_ACCENT_CYAN,
            );
            render_stat_pill(
                ui,
                "DIRTY RATE",
                &format!("{} cells/s", cells_s),
                COLOR_ACCENT_BLUE,
            );
            render_stat_pill(
                ui,
                "TOTAL SENT",
                &format!("{:.2} MB", total_mb),
                COLOR_TEXT_PRIMARY,
            );
            render_stat_pill(
                ui,
                "GRID RESOLUTION",
                &format!("{}x{} ({} Cells)", grid_cfg.cols, grid_cfg.rows, grid_cfg.total_cells),
                COLOR_TEXT_MUTED,
            );
        });
    });
}

fn render_stat_pill(ui: &mut Ui, label: &str, value: &str, val_color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(COLOR_TEXT_MUTED).size(10.0));
        ui.label(
            RichText::new(value)
                .color(val_color)
                .strong()
                .size(11.0),
        );
        ui.add_space(14.0);
    });
}
