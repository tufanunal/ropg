use eframe::egui::{Button, ComboBox, RichText, Slider, Ui, vec2};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::theme::*;
use crate::state::HostSharedState;

pub fn render_encoding_panel(ui: &mut Ui, state: &Arc<HostSharedState>) {
    card_frame().show(ui, |ui| {
        ui.label(
            RichText::new("SCREEN & WEBP ENCODING ENGINE")
                .color(COLOR_ACCENT_CYAN)
                .strong()
                .size(12.0),
        );
        ui.add_space(4.0);

        let mut s = state.settings.lock().unwrap();

        // Grid Resolution & Slicing Mode
        ui.label(RichText::new("Grid Resolution & Slicing Mode").color(COLOR_TEXT_MUTED).size(10.5));
        let prev_grid = s.grid_size.clone();
        ComboBox::from_id_salt("grid_resolution_combo")
            .selected_text(s.grid_size_label())
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.grid_size, "40x40".to_string(), "1920x1080 FHD (48x27 Grid - 1,296 Cells)");
                ui.selectable_value(&mut s.grid_size, "60x60".to_string(), "60x60 (32x18 Grid - 576 Cells)");
                ui.selectable_value(&mut s.grid_size, "120x120".to_string(), "120x120 (16x9 Grid - 144 Cells)");
                ui.selectable_value(&mut s.grid_size, "160x180".to_string(), "160x180 (12x6 Grid - 72 Cells)");
                ui.selectable_value(&mut s.grid_size, "240x270".to_string(), "240x270 (8x4 Grid - 32 Cells)");
                ui.selectable_value(&mut s.grid_size, "384x360".to_string(), "384x360 (5x3 Grid - 15 Cells)");
                ui.selectable_value(&mut s.grid_size, "640x540".to_string(), "640x540 (3x2 Grid - 6 Cells)");
            });

        if s.grid_size != prev_grid {
            s.cell_size = s.get_grid_config().cell_width;
            state.force_full_sync.store(true, Ordering::Relaxed);
        }

        ui.add_space(4.0);

        // Transport Payload Mode Switch
        ui.label(RichText::new("Transport Payload Mode").color(COLOR_TEXT_MUTED).size(10.5));
        ui.horizontal(|ui| {
            let bin_btn = Button::new(
                RichText::new("Mode B (Binary Packet - High Perf)")
                    .color(if s.is_binary_mode { COLOR_SURFACE_INPUT } else { COLOR_TEXT_MUTED })
                    .strong()
                    .size(10.5),
            )
            .fill(if s.is_binary_mode { COLOR_ACCENT_CYAN } else { COLOR_SURFACE_INPUT });

            if ui.add(bin_btn).clicked() {
                s.is_binary_mode = true;
            }

            let json_btn = Button::new(
                RichText::new("Mode A (JSON + Base64)")
                    .color(if !s.is_binary_mode { COLOR_SURFACE_INPUT } else { COLOR_TEXT_MUTED })
                    .strong()
                    .size(10.5),
            )
            .fill(if !s.is_binary_mode { COLOR_ACCENT_CYAN } else { COLOR_SURFACE_INPUT });

            if ui.add(json_btn).clicked() {
                s.is_binary_mode = false;
            }
        });

        ui.add_space(4.0);

        // WebP Quality Slider
        ui.horizontal(|ui| {
            ui.label(RichText::new("WebP Quality (Compression Ratio):").color(COLOR_TEXT_MUTED).size(10.5));
            ui.label(
                RichText::new(format!("{}%", s.quality))
                    .color(COLOR_ACCENT_CYAN)
                    .strong()
                    .size(11.0),
            );
        });
        ui.add(Slider::new(&mut s.quality, 10..=100).show_value(false));

        ui.add_space(4.0);

        // WebP Compression Effort Level
        ui.label(RichText::new("WebP Compression Effort Level").color(COLOR_TEXT_MUTED).size(10.5));
        let prev_comp = s.compression_method;
        ComboBox::from_id_salt("effort_level_combo")
            .selected_text(s.compression_label())
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut s.compression_method, 4, "Level 4 (Fastest, Low Compression)");
                ui.selectable_value(&mut s.compression_method, 5, "Normal Compression (Balanced, Level 5)");
                ui.selectable_value(&mut s.compression_method, 6, "Level 6 (Max Compression)");
            });

        if s.compression_method != prev_comp {
            state.force_full_sync.store(true, Ordering::Relaxed);
        }

        ui.add_space(4.0);

        // Engine Toggles
        ui.checkbox(
            &mut s.dynamic_quality,
            RichText::new("Dynamic Quality Adjustment").strong().size(10.5),
        );
        ui.label(
            RichText::new("  Auto-adjust quality down on high motion scenes to maintain low latency")
                .color(COLOR_TEXT_MUTED)
                .size(8.5),
        );

        ui.add_space(1.0);
        ui.checkbox(
            &mut s.grayscale,
            RichText::new("Grayscale Mode").strong().size(10.5),
        );
        ui.label(
            RichText::new("  Convert WebP tiles to monochrome for up to 60% extra bandwidth savings")
                .color(COLOR_TEXT_MUTED)
                .size(8.5),
        );

        ui.add_space(1.0);
        ui.checkbox(
            &mut s.allow_input,
            RichText::new("Allow Mouse & Keyboard Control").strong().size(10.5),
        );
        ui.label(
            RichText::new("  Enable synthetic input execution via OS-level events")
                .color(COLOR_TEXT_MUTED)
                .size(8.5),
        );

        ui.add_space(4.0);

        // HTTP Adaptive Polling Interval Slider
        ui.horizontal(|ui| {
            ui.label(RichText::new("HTTP Adaptive Polling Interval:").color(COLOR_TEXT_MUTED).size(10.5));
            let req_per_sec = 1000.0 / s.interval_ms as f32;
            ui.label(
                RichText::new(format!("{} ms ({:.0} req/sec)", s.interval_ms, req_per_sec))
                    .color(COLOR_ACCENT_GREEN)
                    .strong()
                    .size(10.5),
            );
        });
        ui.add(Slider::new(&mut s.interval_ms, 50..=1000).show_value(false));

        ui.add_space(6.0);

        let total_cells = s.get_grid_config().total_cells;
        drop(s);

        // Force Full Screen Sync Action Button
        let sync_btn = Button::new(
            RichText::new(format!("Force Full Screen Sync (Re-transmit All {} Cells)", total_cells))
                .strong()
                .size(11.5)
                .color(COLOR_ACCENT_CYAN),
        )
        .fill(COLOR_SURFACE_INPUT)
        .min_size(vec2(ui.available_width(), 28.0));

        if ui.add(sync_btn).clicked() {
            state.force_full_sync.store(true, Ordering::Relaxed);
        }
    });
}
