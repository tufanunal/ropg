pub mod theme;
pub mod header;
pub mod telemetry;
pub mod encoding;
pub mod connection;

use std::sync::Arc;
use std::time::Duration;
use eframe::egui;

use crate::state::HostSharedState;

pub struct RogpHostApp {
    state: Arc<HostSharedState>,
}

impl RogpHostApp {
    pub fn new(cc: &eframe::CreationContext<'_>, state: Arc<HostSharedState>) -> Self {
        theme::setup_custom_theme(&cc.egui_ctx);
        Self { state }
    }
}

impl eframe::App for RogpHostApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint_after(Duration::from_millis(150));

        ui.spacing_mut().item_spacing = egui::vec2(8.0, 5.0);

        header::render_top_bar(ui);
        ui.add_space(3.0);

        header::render_hero_status(ui, &self.state);
        ui.add_space(5.0);

        telemetry::render_telemetry_card(ui, &self.state);
        ui.add_space(5.0);

        // Two-column split layout for Encoding Engine and Relay Connection Panels
        ui.columns(2, |columns| {
            columns[0].vertical(|ui| {
                encoding::render_encoding_panel(ui, &self.state);
            });
            columns[1].vertical(|ui| {
                connection::render_connection_panel(ui, &self.state);
            });
        });
    }
}
