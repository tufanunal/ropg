use eframe::egui::{
    epaint::PathShape, Color32, Grid, Pos2, RichText, Sense, Stroke, Ui, vec2,
};
use std::sync::Arc;

use super::theme::*;
use crate::state::HostSharedState;

pub fn render_telemetry_card(ui: &mut Ui, state: &Arc<HostSharedState>) {
    let grid_cfg = {
        let s = state.settings.lock().unwrap();
        s.get_grid_config()
    };
    let (
        kb_s,
        peak_kb_s,
        cells_s,
        total_bytes,
        last_input,
        last_seq,
        rtt_ms,
        history,
    ) = {
        let m = state.metrics.lock().unwrap();
        (
            m.current_kb_per_sec,
            m.peak_bandwidth_kbps,
            m.dirty_cells_per_sec,
            m.total_bytes_sent,
            m.last_input_event.clone(),
            m.last_input_seq,
            m.rtt_latency_ms,
            m.bandwidth_history.clone(),
        )
    };

    let total_cells = grid_cfg.total_cells;
    let dirty_percent = if total_cells > 0 {
        ((cells_s as f32 / total_cells as f32) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("LIVE TELEMETRY & SYSTEM METRICS")
                .color(COLOR_ACCENT_CYAN)
                .strong()
                .size(13.0),
        );
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            let spark_w = 600.0f32;
            let stats_w = (ui.available_width() - spark_w - 20.0).max(850.0);

            // Left Grid: Sparkline graph
            ui.allocate_ui(vec2(spark_w, 115.0), |ui| {
                ui.label(
                    RichText::new("Upload Bandwidth History (KB/s)")
                        .color(COLOR_TEXT_MUTED)
                        .size(10.0),
                );
                ui.add_space(2.0);
                render_sparkline(ui, &history, kb_s, peak_kb_s, spark_w, 95.0);
            });

            ui.add_space(16.0);

            // Right Grid: 6 Metric Stat Boxes in 3 columns x 2 rows
            let box_width = ((stats_w - 24.0) / 3.0).floor().max(260.0);
            ui.allocate_ui(vec2(stats_w, 115.0), |ui| {
                Grid::new("telemetry_grid_boxes")
                    .num_columns(3)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        render_stat_box(
                            ui,
                            box_width,
                            "Current Bandwidth",
                            &format!("{:.1} KB/s", kb_s),
                            &format!("Peak: {:.1} KB/s", peak_kb_s),
                            COLOR_ACCENT_CYAN,
                        );
                        render_stat_box(
                            ui,
                            box_width,
                            "Grid Cell Updates",
                            &format!("{} cells/s", cells_s),
                            &format!("{:.1}% of total screen", dirty_percent),
                            COLOR_ACCENT_BLUE,
                        );
                        render_stat_box(
                            ui,
                            box_width,
                            "Total Payload Sent",
                            &format!("{:.2} MB", total_bytes as f64 / 1_048_576.0),
                            &format!("{} bytes", total_bytes),
                            COLOR_TEXT_PRIMARY,
                        );
                        ui.end_row();

                        render_stat_box(
                            ui,
                            box_width,
                            "Last Input Command",
                            &last_input,
                            &format!("seq: {}", last_seq),
                            COLOR_ACCENT_GREEN,
                        );
                        render_stat_box(
                            ui,
                            box_width,
                            "Grid Partition",
                            &format!("{} cells", grid_cfg.total_cells),
                            &format!("{} cols x {} rows", grid_cfg.cols, grid_cfg.rows),
                            COLOR_TEXT_PRIMARY,
                        );
                        render_stat_box(
                            ui,
                            box_width,
                            "Network Latency",
                            &format!("{} ms", rtt_ms),
                            "HTTP GET / POST RTT",
                            COLOR_ACCENT_GREEN,
                        );
                        ui.end_row();
                    });
            });
        });
    });
}

fn render_sparkline(ui: &mut Ui, history: &[f32], current_kb_s: f32, peak_kb_s: f32, width: f32, height: f32) {
    let desired_size = vec2(width, height);
    let (rect, _response) = ui.allocate_exact_size(desired_size, Sense::hover());
    let painter = ui.painter_at(rect);

    // Background and subtle border
    painter.rect_filled(rect, 6.0, COLOR_SURFACE_INPUT);
    painter.rect_stroke(rect, 6.0, Stroke::new(1.0, COLOR_BORDER_CARD), eframe::egui::StrokeKind::Inside);

    // Subtle horizontal grid guidelines
    for i in 1..4 {
        let y = rect.top() + (rect.height() / 4.0) * i as f32;
        painter.line_segment(
            [Pos2::new(rect.left() + 6.0, y), Pos2::new(rect.right() - 6.0, y)],
            Stroke::new(0.5, Color32::from_rgb(20, 28, 42)),
        );
    }

    if history.is_empty() {
        return;
    }

    // Determine scale: max bandwidth with minimum ceiling of 50.0 KB/s
    let mut max_val = 50.0f32;
    for &v in history {
        if v > max_val {
            max_val = v;
        }
    }
    if peak_kb_s > max_val {
        max_val = peak_kb_s;
    }
    max_val *= 1.12;

    let pad_x = 8.0;
    let pad_y = 6.0;
    let plot_w = (rect.width() - pad_x * 2.0).max(1.0);
    let plot_h = (rect.height() - pad_y * 2.0).max(1.0);

    let count = history.len();
    let mut points = Vec::with_capacity(count);

    for (i, &val) in history.iter().enumerate() {
        let x = rect.left() + pad_x + (i as f32 / (count - 1).max(1) as f32) * plot_w;
        let clamped_val = val.clamp(0.0, max_val);
        let y = rect.bottom() - pad_y - (clamped_val / max_val) * plot_h;
        points.push(Pos2::new(x, y));
    }

    // White/cyan filled gradient polygon matching reference image
    if points.len() >= 2 {
        let mut fill_points = Vec::with_capacity(points.len() + 2);
        fill_points.push(Pos2::new(points[0].x, rect.bottom() - pad_y));
        fill_points.extend(points.iter().copied());
        fill_points.push(Pos2::new(points.last().unwrap().x, rect.bottom() - pad_y));

        painter.add(PathShape::convex_polygon(
            fill_points,
            Color32::from_rgba_unmultiplied(225, 250, 255, 230),
            Stroke::NONE,
        ));

        // Line stroke
        painter.add(PathShape::line(
            points.clone(),
            Stroke::new(1.8, COLOR_ACCENT_CYAN),
        ));

        // Latest value dot
        if let Some(&last_pt) = points.last() {
            painter.circle_filled(last_pt, 4.0, COLOR_ACCENT_CYAN);
        }
    }

    // Telemetry text overlay
    painter.text(
        Pos2::new(rect.left() + 8.0, rect.top() + 6.0),
        eframe::egui::Align2::LEFT_TOP,
        format!("LIVE: {:.1} KB/s", current_kb_s),
        eframe::egui::FontId::monospace(10.5),
        Color32::from_rgb(15, 23, 42),
    );
    painter.text(
        Pos2::new(rect.right() - 8.0, rect.top() + 6.0),
        eframe::egui::Align2::RIGHT_TOP,
        format!("MAX: {:.1} KB/s", max_val),
        eframe::egui::FontId::monospace(9.5),
        COLOR_TEXT_MUTED,
    );
}

fn render_stat_box(
    ui: &mut Ui,
    width: f32,
    title: &str,
    primary_val: &str,
    subtitle: &str,
    val_color: Color32,
) {
    inner_box_frame().show(ui, |ui| {
        ui.set_width(width);
        ui.label(RichText::new(title).color(COLOR_TEXT_MUTED).size(10.0));
        ui.add_space(1.0);
        ui.label(
            RichText::new(primary_val)
                .color(val_color)
                .strong()
                .size(13.5),
        );
        ui.label(RichText::new(subtitle).color(COLOR_TEXT_MUTED).size(9.0));
    });
}
