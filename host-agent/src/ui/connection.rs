use eframe::egui::{Button, Color32, CornerRadius, Frame, Margin, RichText, Stroke, TextEdit, Ui, vec2};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use super::theme::*;
use crate::state::HostSharedState;

pub fn render_connection_panel(ui: &mut Ui, state: &Arc<HostSharedState>) {
    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("RELAY SERVER & CONNECTION CONFIG")
                .color(COLOR_ACCENT_CYAN)
                .strong()
                .size(13.0),
        );
        ui.add_space(8.0);

        let mut s = state.settings.lock().unwrap();

        // Host Machine Identifier
        ui.label(RichText::new("Host Machine Identifier").color(COLOR_TEXT_MUTED).size(11.0));
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut s.host_id)
                    .hint_text("host-1")
                    .desired_width(ui.available_width() - 100.0),
            );
            if ui.add(Button::new(RichText::new("Random").size(11.0)).min_size(vec2(90.0, 26.0))).clicked() {
                let random_id = format!(
                    "host-{}",
                    (std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                        % 900
                        + 100)
                );
                s.host_id = random_id;
            }
        });

        ui.add_space(8.0);

        // Relay Server Endpoint
        ui.label(RichText::new("Relay Server Endpoint").color(COLOR_TEXT_MUTED).size(11.0));
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut s.relay_url)
                    .hint_text("https://relay.example.com")
                    .desired_width(ui.available_width() - 100.0),
            );
            if ui.add(Button::new(RichText::new("Test").size(11.0)).min_size(vec2(90.0, 26.0))).clicked() {
                let test_url = s.relay_url.clone();
                let test_state = state.clone();
                std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::builder()
                        .timeout(std::time::Duration::from_millis(2000))
                        .build();
                    let start = Instant::now();
                    match client {
                        Ok(c) => match c.get(format!("{}/api/host/ping", test_url)).send() {
                            Ok(resp) => {
                                let rtt = start.elapsed().as_millis();
                                let mut m = test_state.metrics.lock().unwrap();
                                m.relay_test_status = Some(format!(
                                    "HTTP {} ({}ms)",
                                    resp.status().as_u16(),
                                    rtt
                                ));
                            }
                            Err(e) => {
                                let mut m = test_state.metrics.lock().unwrap();
                                m.relay_test_status = Some(format!("Failed: {}", e));
                            }
                        },
                        Err(e) => {
                            let mut m = test_state.metrics.lock().unwrap();
                            m.relay_test_status = Some(format!("Error: {}", e));
                        }
                    }
                });
            }
        });

        // Test status notification if available
        let test_status = { state.metrics.lock().unwrap().relay_test_status.clone() };
        if let Some(status_str) = test_status {
            ui.label(
                RichText::new(&status_str)
                    .color(if status_str.starts_with("HTTP 200") {
                        COLOR_ACCENT_GREEN
                    } else {
                        COLOR_ACCENT_RED
                    })
                    .size(10.0),
            );
        }

        ui.add_space(8.0);

        // Secret Authentication Token
        ui.label(RichText::new("Secret Authentication Token").color(COLOR_TEXT_MUTED).size(11.0));
        ui.horizontal(|ui| {
            let hide_pw = !s.show_token;
            ui.add(
                TextEdit::singleline(&mut s.token)
                    .password(hide_pw)
                    .desired_width(ui.available_width() - 100.0),
            );
            let toggle_label = if s.show_token { "Hide" } else { "Show" };
            if ui.add(Button::new(RichText::new(toggle_label).size(11.0)).min_size(vec2(90.0, 26.0))).clicked() {
                s.show_token = !s.show_token;
            }
        });

        ui.add_space(10.0);

        let (is_conn, rtt) = {
            let m = state.metrics.lock().unwrap();
            (m.is_connected, m.rtt_latency_ms)
        };

        // Relay Handshake Status Card (green border matching reference)
        Frame::new()
            .fill(COLOR_SURFACE_INPUT)
            .stroke(Stroke::new(1.0, COLOR_ACCENT_GREEN))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let dot_color = if is_conn { COLOR_ACCENT_GREEN } else { COLOR_ACCENT_BLUE };
                    let (dot_rect, _) = ui.allocate_exact_size(vec2(9.0, 9.0), eframe::egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 4.0, dot_color);
                    ui.add_space(3.0);

                    let title_text = if is_conn {
                        "Relay Server Handshake Verified"
                    } else {
                        "Relay Server Handshake Pending..."
                    };
                    ui.label(
                        RichText::new(title_text)
                            .color(COLOR_ACCENT_GREEN)
                            .strong()
                            .size(12.0),
                    );
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new("Connected to Relay Server via HTTP Polling")
                        .color(COLOR_TEXT_MUTED)
                        .size(10.5),
                );
                ui.label(
                    RichText::new(format!(
                        "RTT: {}ms - Session TTL: 4.0s Sliding Window - Protocol: ROGP v1",
                        rtt
                    ))
                    .color(COLOR_TEXT_MUTED)
                    .size(9.5),
                );
            });

        ui.add_space(10.0);

        // Security & Synthetic Input Controls Info Box (amber border)
        Frame::new()
            .fill(Color32::from_rgb(18, 15, 24))
            .stroke(Stroke::new(1.0, Color32::from_rgb(70, 48, 25)))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Security & Synthetic Input Controls")
                        .color(COLOR_ACCENT_AMBER)
                        .strong()
                        .size(11.5),
                );
                ui.add_space(3.0);
                ui.label(
                    RichText::new("- Synthetic Hardware Cursor Exclusion: ACTIVE (Prevents screen dirtying)")
                        .color(COLOR_TEXT_MUTED)
                        .size(9.5),
                );
                ui.label(
                    RichText::new("- Input Rate Limiting: 50ms Drag Throttle / 200ms Hover Rest")
                        .color(COLOR_TEXT_MUTED)
                        .size(9.5),
                );
                ui.label(
                    RichText::new("- Emergency Input Kill-Switch: Press Ctrl+Shift+Esc to disable")
                        .color(COLOR_TEXT_MUTED)
                        .size(9.5),
                );
                ui.label(
                    RichText::new("- Session Security: Double-click sequence protection enabled")
                        .color(COLOR_TEXT_MUTED)
                        .size(9.5),
                );
            });

        ui.add_space(14.0);

        // Save & Disconnect Footer Buttons matching reference
        ui.horizontal(|ui| {
            let save_btn = Button::new(
                RichText::new("Save & Apply Settings")
                    .color(COLOR_SURFACE_INPUT)
                    .strong()
                    .size(12.5),
            )
            .fill(COLOR_ACCENT_CYAN)
            .min_size(vec2(180.0, 36.0));

            if ui.add(save_btn).clicked() {
                s.save();
                state.force_full_sync.store(true, Ordering::Relaxed);
            }

            ui.add_space(16.0);

            let dc_btn = Button::new(
                RichText::new("Disconnect Agent")
                    .color(COLOR_ACCENT_RED)
                    .strong()
                    .size(12.5),
            )
            .fill(COLOR_SURFACE_INPUT)
            .min_size(vec2(160.0, 36.0));

            if ui.add(dc_btn).clicked() {
                state.is_running.store(false, Ordering::Relaxed);
            }
        });
    });
}
