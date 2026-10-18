use eframe::egui::{Color32, Context, CornerRadius, Frame, Margin, Stroke, Visuals};

pub const COLOR_BG_CANVAS: Color32 = Color32::from_rgb(9, 13, 22);
pub const COLOR_SURFACE_CARD: Color32 = Color32::from_rgb(19, 27, 42);
pub const COLOR_SURFACE_INPUT: Color32 = Color32::from_rgb(12, 17, 28);
pub const COLOR_BORDER_CARD: Color32 = Color32::from_rgb(33, 45, 66);

pub const COLOR_ACCENT_CYAN: Color32 = Color32::from_rgb(0, 240, 255);
pub const COLOR_ACCENT_GREEN: Color32 = Color32::from_rgb(0, 255, 135);
pub const COLOR_ACCENT_RED: Color32 = Color32::from_rgb(255, 51, 102);
pub const COLOR_ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);
pub const COLOR_ACCENT_AMBER: Color32 = Color32::from_rgb(245, 158, 11);

pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 244, 248);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(139, 155, 180);

pub fn setup_custom_theme(ctx: &Context) {
    ctx.set_zoom_factor(1.0);
    let mut visuals = Visuals::dark();

    visuals.override_text_color = Some(COLOR_TEXT_PRIMARY);
    visuals.panel_fill = COLOR_BG_CANVAS;
    visuals.window_fill = COLOR_BG_CANVAS;

    visuals.widgets.noninteractive.bg_fill = COLOR_SURFACE_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, COLOR_BORDER_CARD);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = COLOR_SURFACE_INPUT;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, COLOR_BORDER_CARD);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(5);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(25, 35, 55);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, COLOR_ACCENT_CYAN);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(5);

    visuals.widgets.active.bg_fill = COLOR_ACCENT_CYAN;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::BLACK);
    visuals.widgets.active.corner_radius = CornerRadius::same(5);

    ctx.set_visuals(visuals);
}

pub fn card_frame() -> Frame {
    Frame::new()
        .fill(COLOR_SURFACE_CARD)
        .stroke(Stroke::new(1.0, COLOR_BORDER_CARD))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(14, 10))
}

pub fn inner_box_frame() -> Frame {
    Frame::new()
        .fill(COLOR_SURFACE_INPUT)
        .stroke(Stroke::new(1.0, COLOR_BORDER_CARD))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(10, 6))
}

pub fn pill_frame() -> Frame {
    Frame::new()
        .fill(COLOR_SURFACE_INPUT)
        .stroke(Stroke::new(1.0, COLOR_BORDER_CARD))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(8, 5))
}
