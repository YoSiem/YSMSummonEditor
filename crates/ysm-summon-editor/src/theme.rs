//! Visual identity: palette, fonts and widget styling.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, TextStyle, Vec2,
};

/// The dark slate palette of the original editor, plus semantic colours.
pub mod color {
    use eframe::egui::Color32;

    pub const WINDOW_BG: Color32 = Color32::from_rgb(0x17, 0x1A, 0x1F);
    pub const PANEL_BG: Color32 = Color32::from_rgb(0x1E, 0x23, 0x2B);
    pub const PANEL_ALT: Color32 = Color32::from_rgb(0x24, 0x2A, 0x33);
    pub const BORDER: Color32 = Color32::from_rgb(0x3A, 0x40, 0x4B);
    pub const CONTROL_BG: Color32 = Color32::from_rgb(0x2B, 0x31, 0x3B);
    pub const CONTROL_BORDER: Color32 = Color32::from_rgb(0x43, 0x4C, 0x59);
    pub const INPUT_BG: Color32 = Color32::from_rgb(0x14, 0x17, 0x1C);

    pub const TEXT: Color32 = Color32::from_rgb(0xEC, 0xEF, 0xF4);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xA5, 0xAE, 0xB9);
    pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x6E, 0x78, 0x86);

    pub const ACCENT: Color32 = Color32::from_rgb(0x5C, 0x72, 0x8A);
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x6B, 0x84, 0x9F);
    pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(0x9C, 0xC2, 0xEA);
    pub const PRIMARY: Color32 = Color32::from_rgb(0x3D, 0x7A, 0xB8);

    pub const ROW_EVEN: Color32 = Color32::from_rgb(0x25, 0x2C, 0x36);
    pub const ROW_HOVER: Color32 = Color32::from_rgb(0x2F, 0x38, 0x45);
    pub const ROW_DIRTY: Color32 = Color32::from_rgb(0x5A, 0x2F, 0x35);
    pub const ROW_SELECTED: Color32 = Color32::from_rgb(0x4A, 0x5F, 0x78);

    pub const SUCCESS: Color32 = Color32::from_rgb(0x6F, 0xCF, 0x97);
    pub const WARNING: Color32 = Color32::from_rgb(0xF2, 0xC9, 0x4C);
    pub const DANGER: Color32 = Color32::from_rgb(0xEB, 0x6B, 0x6B);
    pub const DANGER_BG: Color32 = Color32::from_rgb(0x8A, 0x33, 0x3A);
    pub const INFO: Color32 = Color32::from_rgb(0x8E, 0xCA, 0xE6);
    pub const DIRTY_MARK: Color32 = Color32::from_rgb(0xF2, 0x99, 0x4A);
}

pub const RADIUS: u8 = 6;
pub const ICON_SIZE: f32 = 36.0;
/// Height of grid rows that carry a 36 px icon.
pub const ROW_HEIGHT: f32 = 40.0;

/// Font family used for emphasis; falls back to the regular face when no bold font is installed.
pub fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, bold_family())
}

/// A rounded, bordered container for a section of the window.
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(color::PANEL_BG)
        .stroke(Stroke::new(1.0, color::BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
}

/// Applies fonts and style to the whole application.
pub fn apply(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(apply_style);
}

fn apply_style(style: &mut egui::Style) {
    style.text_styles = [
        (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
        (TextStyle::Heading, bold(20.0)),
    ]
    .into();

    let spacing = &mut style.spacing;
    spacing.item_spacing = Vec2::new(8.0, 6.0);
    spacing.button_padding = Vec2::new(10.0, 5.0);
    spacing.interact_size.y = 28.0;
    spacing.window_margin = Margin::same(14);
    spacing.menu_margin = Margin::same(6);
    spacing.scroll.bar_width = 10.0;
    spacing.scroll.floating = false;

    let radius = CornerRadius::same(RADIUS);
    let visuals = &mut style.visuals;
    *visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(color::TEXT);
    visuals.weak_text_color = Some(color::TEXT_MUTED);
    visuals.panel_fill = color::WINDOW_BG;
    visuals.window_fill = color::PANEL_BG;
    visuals.window_stroke = Stroke::new(1.0, color::BORDER);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = radius;
    visuals.extreme_bg_color = color::INPUT_BG;
    visuals.faint_bg_color = color::ROW_EVEN;
    visuals.code_bg_color = color::INPUT_BG;
    visuals.hyperlink_color = color::ACCENT_BRIGHT;
    visuals.warn_fg_color = color::WARNING;
    visuals.error_fg_color = color::DANGER;
    visuals.selection.bg_fill = color::ROW_SELECTED;
    visuals.selection.stroke = Stroke::new(1.0, color::ACCENT_BRIGHT);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = color::PANEL_BG;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, color::BORDER);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, color::TEXT);
    widgets.noninteractive.corner_radius = radius;

    widgets.inactive.bg_fill = color::CONTROL_BG;
    widgets.inactive.weak_bg_fill = color::CONTROL_BG;
    widgets.inactive.bg_stroke = Stroke::new(1.0, color::CONTROL_BORDER);
    widgets.inactive.fg_stroke = Stroke::new(1.0, color::TEXT);
    widgets.inactive.corner_radius = radius;

    widgets.hovered.bg_fill = color::ACCENT;
    widgets.hovered.weak_bg_fill = color::ACCENT;
    widgets.hovered.bg_stroke = Stroke::new(1.0, color::ACCENT_HOVER);
    widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    widgets.hovered.corner_radius = radius;
    widgets.hovered.expansion = 0.0;

    widgets.active.bg_fill = color::ACCENT_HOVER;
    widgets.active.weak_bg_fill = color::ACCENT_HOVER;
    widgets.active.bg_stroke = Stroke::new(1.0, color::ACCENT_BRIGHT);
    widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    widgets.active.corner_radius = radius;
    widgets.active.expansion = 0.0;

    widgets.open.bg_fill = color::CONTROL_BG;
    widgets.open.weak_bg_fill = color::ACCENT;
    widgets.open.bg_stroke = Stroke::new(1.0, color::ACCENT_HOVER);
    widgets.open.fg_stroke = Stroke::new(1.0, color::TEXT);
    widgets.open.corner_radius = radius;
}

/// egui's default fonts, preceded by Segoe UI when available, with Phosphor
/// icons and a Korean-capable fallback for game strings.
fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let system_font = |file: &str| {
        let windows = std::env::var_os("WINDIR").map(std::path::PathBuf::from)?;
        std::fs::read(windows.join("Fonts").join(file)).ok()
    };

    if let Some(bytes) = system_font("segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), Arc::new(FontData::from_owned(bytes)));
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "segoe".into());
    }

    // The bold family is the bold face followed by everything the regular family has.
    let mut bold_chain = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(bytes) = system_font("segoeuib.ttf") {
        fonts.font_data.insert("segoe-bold".into(), Arc::new(FontData::from_owned(bytes)));
        bold_chain.insert(0, "segoe-bold".into());
    }
    fonts.families.insert(bold_family(), bold_chain);

    if let Some(bytes) = system_font("malgun.ttf") {
        fonts.font_data.insert("malgun".into(), Arc::new(FontData::from_owned(bytes)));
        for family in fonts.families.values_mut() {
            family.push("malgun".into());
        }
    }

    fonts
}

/// The window icon, decoded from the embedded `.ico`.
pub fn window_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(include_bytes!("../../../assets/Logo.ico")).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData { rgba: image.into_raw(), width, height })
}
