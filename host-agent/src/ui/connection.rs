use eframe::egui::{Button, RichText, TextEdit, Ui, vec2};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use super::theme::*;
use crate::state::HostSharedState;

pub fn render_connection_panel(ui: &mut Ui, state: &Arc<HostSharedState>) {
    card_frame().show(ui, |ui| {
        ui.heading(
            RichText::new("🌐 Relay Server & Connection Configuration")
                .color(COLOR_TEXT_PRIMARY)
                .strong()
                .size(15.0),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Manage device credentials, relay endpoints, and authentication")
                .color(COLOR_TEXT_MUTED)
                .size(11.0),
        );

        ui.add_space(12.0);

        let mut s = state.settings.lock().unwrap();

        // Host Machine Identifier
        ui.label(RichText::new("Host Machine Identifier").strong().size(12.0));
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut s.host_id)
                    .hint_text("e.g. host-1")
                    .desired_width(200.0),
            );
            if ui.button("🎲 Random").clicked() {
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

        ui.add_space(12.0);

        // Relay Server Endpoint URL
        ui.label(RichText::new("Relay Server Endpoint URL").strong().size(12.0));
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut s.relay_url)
                    .hint_text("http://127.0.0.1:8080")
                    .desired_width(200.0),
            );
            if ui.button("Test 🧪").clicked() {
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
                                    "✅ Verified HTTP {} ({}ms)",
                                    resp.status().as_u16(),
                                    rtt
                                ));
                            }
                            Err(e) => {
                                let mut m = test_state.metrics.lock().unwrap();
                                m.relay_test_status = Some(format!("❌ Ping failed: {}", e));
                            }
                        },
                        Err(e) => {
                            let mut m = test_state.metrics.lock().unwrap();
                            m.relay_test_status = Some(format!("❌ Client init: {}", e));
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
                    .color(if status_str.starts_with("✅") {
                        COLOR_ACCENT_GREEN
                    } else {
                        COLOR_ACCENT_RED
                    })
                    .size(11.0),
            );
        }

        ui.add_space(12.0);

        // Secret Auth Token
        ui.label(RichText::new("Secret Authentication Token").strong().size(12.0));
        ui.horizontal(|ui| {
            let hide_pw = !s.show_token;
            ui.add(
                TextEdit::singleline(&mut s.token)
                    .password(hide_pw)
                    .desired_width(200.0),
            );
            let toggle_label = if s.show_token { "🙈 Hide" } else { "👁️ Show" };
            if ui.button(toggle_label).clicked() {
                s.show_token = !s.show_token;
            }
        });

        ui.add_space(14.0);

        let (is_conn, rtt) = {
            let m = state.metrics.lock().unwrap();
            (m.is_connected, m.rtt_latency_ms)
        };

        // Relay Handshake Status Card
        inner_box_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                if is_conn {
                    ui.label(
                        RichText::new("✅ Relay Handshake Verified")
                            .color(COLOR_ACCENT_GREEN)
                            .strong()
                            .size(12.0),
                    );
                } else {
                    ui.label(
                        RichText::new("⏳ Relay Handshake Pending...")
                            .color(COLOR_ACCENT_BLUE)
                            .strong()
                            .size(12.0),
                    );
                }
            });
            ui.label(
                RichText::new(format!(
                    "Connected via HTTP Polling Engine • RTT: {}ms",
                    rtt
                ))
                .color(COLOR_TEXT_MUTED)
                .size(11.0),
            );
            ui.label(
                RichText::new("Sliding TTL Eviction: 4.0s active window")
                    .color(COLOR_TEXT_MUTED)
                    .size(10.0),
            );
        });

        ui.add_space(12.0);

        // Security & Synthetic Input Controls Info Box
        inner_box_frame().show(ui, |ui| {
            ui.label(
                RichText::new("🔒 Security & Synthetic Input Controls")
                    .strong()
                    .size(12.0),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("• Hardware Cursor Exclusion: ACTIVE (prevents dirtying cells)")
                    .color(COLOR_TEXT_MUTED)
                    .size(10.0),
            );
            ui.label(
                RichText::new("• Input Throttling: 50ms Drag / 200ms Hover Rest")
                    .color(COLOR_TEXT_MUTED)
                    .size(10.0),
            );
            ui.label(
                RichText::new("• Emergency Input Kill-Switch: Ctrl+Shift+Esc")
                    .color(COLOR_TEXT_MUTED)
                    .size(10.0),
            );
            ui.label(
                RichText::new("• Sequence ID duplication prevention enabled")
                    .color(COLOR_TEXT_MUTED)
                    .size(10.0),
            );
        });

        ui.add_space(16.0);

        // Save & Disconnect Footer Buttons
        ui.horizontal(|ui| {
            let save_btn = Button::new(
                RichText::new("💾 Save & Apply Settings")
                    .color(COLOR_SURFACE_INPUT)
                    .strong()
                    .size(13.0),
            )
            .fill(COLOR_ACCENT_CYAN)
            .min_size(vec2(160.0, 38.0));

            if ui.add(save_btn).clicked() {
                s.save();
                state.force_full_sync.store(true, Ordering::Relaxed);
            }

            let dc_btn = Button::new(
                RichText::new("🔌 Disconnect Agent")
                    .color(COLOR_ACCENT_RED)
                    .strong()
                    .size(13.0),
            )
            .fill(COLOR_SURFACE_INPUT)
            .min_size(vec2(140.0, 38.0));

            if ui.add(dc_btn).clicked() {
                state.is_running.store(false, Ordering::Relaxed);
            }
        });
    });
}
