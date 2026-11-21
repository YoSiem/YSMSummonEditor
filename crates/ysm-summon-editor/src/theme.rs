//! Visual identity: palette, type scale, spacing grid and widget styling.
//!
//! Everything visual is defined here once. Views use these tokens and never
//! invent a colour, a font size or a spacing value of their own:
//!
//! - **Surfaces** are layered: shell < content < control < menu.
//! - **Spacing** comes from a 4 px grid: 4, 8, 12, 16, 20, 24, 32, 40.
//! - **Type** has four sizes: 18 title, 15 section, 13 body, 12 caption.
//! - **Controls** are 32 px high with a 6 px radius; cards and dialogs use 8.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{
    self, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, TextStyle, Vec2,
};

pub mod color {
    use eframe::egui::Color32;

    // Surfaces, darkest to lightest.
    /// Application chrome: toolbar, status bar and the gaps between cards.
    pub const WINDOW_BG: Color32 = Color32::from_rgb(0x14, 0x16, 0x1B);
    /// Cards and dialogs: the surface the user works on.
    pub const PANEL_BG: Color32 = Color32::from_rgb(0x1B, 0x1E, 0x24);
    /// Inputs and secondary buttons at rest.
    pub const CONTROL_BG: Color32 = Color32::from_rgb(0x23, 0x27, 0x32);
    pub const CONTROL_HOVER: Color32 = Color32::from_rgb(0x2A, 0x2F, 0x3B);
    pub const CONTROL_PRESSED: Color32 = Color32::from_rgb(0x1F, 0x23, 0x2C);
    /// Menus, popovers, tooltips and toasts.
    pub const PANEL_ALT: Color32 = Color32::from_rgb(0x25, 0x2A, 0x35);
    /// Recessed areas inside a card or dialog: lists, code, charts.
    pub const INPUT_BG: Color32 = Color32::from_rgb(0x14, 0x16, 0x1B);

    // Borders.
    /// Hairlines: separators and card outlines.
    pub const BORDER: Color32 = Color32::from_rgb(0x2A, 0x2E, 0x38);
    /// Control outlines at rest.
    pub const CONTROL_BORDER: Color32 = Color32::from_rgb(0x33, 0x39, 0x47);
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x41, 0x4A, 0x5C);
    pub const BORDER_FOCUS: Color32 = Color32::from_rgb(0x5B, 0x94, 0xFF);

    // Text.
    pub const TEXT: Color32 = Color32::from_rgb(0xE8, 0xEB, 0xF0);
    /// Secondary text: captions, help, inactive items.
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xA9, 0xB1, 0xC0);
    /// Placeholders and de-emphasised values such as zeros in a grid.
    pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x6C, 0x74, 0x84);
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x59, 0x60, 0x72);
    /// Links, active tabs and selected items.
    pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(0x79, 0xA8, 0xFF);

    // Accent.
    pub const PRIMARY: Color32 = Color32::from_rgb(0x2F, 0x6F, 0xED);
    pub const PRIMARY_DISABLED: Color32 = Color32::from_rgb(0x22, 0x30, 0x55);
    /// Fill of the active tab or segment.
    pub const ACCENT_SUBTLE: Color32 = Color32::from_rgba_premultiplied(17, 29, 54, 56);

    // Rows. The tints are translucent so they sit on any surface.
    pub const ROW_STRIPE: Color32 = Color32::from_rgba_premultiplied(5, 5, 5, 5);
    pub const ROW_HOVER: Color32 = Color32::from_rgb(0x2A, 0x2F, 0x3B);
    pub const ROW_SELECTED: Color32 = Color32::from_rgba_premultiplied(25, 41, 77, 80);
    /// A row with staged changes.
    pub const ROW_DIRTY: Color32 = Color32::from_rgba_premultiplied(37, 27, 6, 45);

    // Semantic.
    pub const SUCCESS: Color32 = Color32::from_rgb(0x3F, 0xB9, 0x50);
    pub const WARNING: Color32 = Color32::from_rgb(0xD2, 0x99, 0x22);
    pub const DANGER: Color32 = Color32::from_rgb(0xF8, 0x51, 0x49);
    /// Fill of destructive buttons; dark enough for white text.
    pub const DANGER_BG: Color32 = Color32::from_rgb(0xB6, 0x23, 0x24);
    pub const INFO: Color32 = Color32::from_rgb(0x79, 0xA8, 0xFF);
    /// Values and markers of staged, unsaved edits.
    pub const DIRTY_MARK: Color32 = Color32::from_rgb(0xD2, 0x99, 0x22);
}

/// Spacing grid. No other spacing values are used anywhere.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// Height of every button, input and tab.
pub const CONTROL_HEIGHT: f32 = 32.0;
/// Minimum width of a text button.
pub const BUTTON_MIN_WIDTH: f32 = 88.0;
pub const RADIUS: u8 = 6;
pub const CARD_RADIUS: u8 = 8;
/// Game icons are 36 px squares.
pub const ICON_SIZE: f32 = 36.0;
/// Height of grid rows that carry an icon.
pub const ROW_HEIGHT: f32 = 40.0;
/// Height of list rows that carry an icon and two lines of text.
pub const LIST_ROW_HEIGHT: f32 = 48.0;
/// Height of a grid header row.
pub const HEADER_HEIGHT: f32 = 28.0;

pub const TITLE_SIZE: f32 = 18.0;
pub const SECTION_SIZE: f32 = 15.0;
pub const BODY_SIZE: f32 = 13.0;
pub const CAPTION_SIZE: f32 = 12.0;
/// Size of icon glyphs inside controls.
pub const GLYPH_SIZE: f32 = 16.0;

/// Font family used for emphasis; falls back to the regular face when no bold font is installed.
pub fn bold_family() -> FontFamily {
    FontFamily::Name("bold".into())
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, bold_family())
}

pub fn body() -> FontId {
    FontId::proportional(BODY_SIZE)
}

pub fn caption() -> FontId {
    FontId::proportional(CAPTION_SIZE)
}

/// A rounded container for a section of the window.
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(color::PANEL_BG)
        .stroke(Stroke::new(1.0, color::BORDER))
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(Margin::same(space::MD as i8))
}

/// A recessed area inside a card or dialog.
pub fn well() -> egui::Frame {
    egui::Frame::new()
        .fill(color::INPUT_BG)
        .stroke(Stroke::new(1.0, color::BORDER))
        .corner_radius(CornerRadius::same(RADIUS))
        .inner_margin(Margin::same(space::XS as i8))
}

/// Applies fonts and style to the whole application.
pub fn apply(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(apply_style);
}

fn apply_style(style: &mut egui::Style) {
    style.text_styles = [
        (TextStyle::Small, FontId::new(CAPTION_SIZE, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(BODY_SIZE, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(BODY_SIZE, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(BODY_SIZE, FontFamily::Monospace)),
        (TextStyle::Heading, bold(TITLE_SIZE)),
    ]
    .into();

    let spacing = &mut style.spacing;
    spacing.item_spacing = Vec2::splat(space::SM);
    spacing.button_padding = Vec2::new(space::MD, 6.0);
    spacing.interact_size = Vec2::new(40.0, CONTROL_HEIGHT);
    spacing.window_margin = Margin::same(space::XL as i8);
    spacing.menu_margin = Margin::same(space::XS as i8);
    spacing.icon_width = 16.0;
    spacing.icon_spacing = space::SM;
    spacing.scroll = egui::style::ScrollStyle::floating();
    spacing.scroll.bar_width = 8.0;
    spacing.scroll.dormant_handle_opacity = 0.5;
    spacing.scroll.active_handle_opacity = 0.7;
    spacing.scroll.interact_handle_opacity = 1.0;

    let radius = CornerRadius::same(RADIUS);
    let visuals = &mut style.visuals;
    *visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(color::TEXT);
    visuals.weak_text_color = Some(color::TEXT_MUTED);
    visuals.panel_fill = color::WINDOW_BG;
    visuals.window_fill = color::PANEL_BG;
    visuals.window_stroke = Stroke::new(1.0, color::CONTROL_BORDER);
    visuals.window_corner_radius = CornerRadius::same(CARD_RADIUS);
    visuals.menu_corner_radius = CornerRadius::same(CARD_RADIUS);
    visuals.extreme_bg_color = color::CONTROL_BG;
    visuals.text_edit_bg_color = Some(color::CONTROL_BG);
    visuals.faint_bg_color = color::ROW_STRIPE;
    visuals.code_bg_color = color::INPUT_BG;
    visuals.hyperlink_color = color::ACCENT_BRIGHT;
    visuals.warn_fg_color = color::WARNING;
    visuals.error_fg_color = color::DANGER;
    visuals.selection.bg_fill = color::ROW_SELECTED;
    visuals.selection.stroke = Stroke::new(1.0, color::BORDER_FOCUS);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = color::PANEL_BG;
    widgets.noninteractive.weak_bg_fill = color::PANEL_BG;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, color::BORDER);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, color::TEXT);
    widgets.noninteractive.corner_radius = radius;

    widgets.inactive.bg_fill = color::CONTROL_BG;
    widgets.inactive.weak_bg_fill = color::CONTROL_BG;
    widgets.inactive.bg_stroke = Stroke::new(1.0, color::CONTROL_BORDER);
    widgets.inactive.fg_stroke = Stroke::new(1.0, color::TEXT);
    widgets.inactive.corner_radius = radius;
    widgets.inactive.expansion = 0.0;

    widgets.hovered.bg_fill = color::CONTROL_HOVER;
    widgets.hovered.weak_bg_fill = color::CONTROL_HOVER;
    widgets.hovered.bg_stroke = Stroke::new(1.0, color::BORDER_STRONG);
    widgets.hovered.fg_stroke = Stroke::new(1.0, color::TEXT);
    widgets.hovered.corner_radius = radius;
    widgets.hovered.expansion = 0.0;

    widgets.active.bg_fill = color::CONTROL_PRESSED;
    widgets.active.weak_bg_fill = color::CONTROL_PRESSED;
    widgets.active.bg_stroke = Stroke::new(1.0, color::BORDER_FOCUS);
    widgets.active.fg_stroke = Stroke::new(1.0, color::TEXT);
    widgets.active.corner_radius = radius;
    widgets.active.expansion = 0.0;

    widgets.open.bg_fill = color::CONTROL_HOVER;
    widgets.open.weak_bg_fill = color::CONTROL_HOVER;
    widgets.open.bg_stroke = Stroke::new(1.0, color::BORDER_STRONG);
    widgets.open.fg_stroke = Stroke::new(1.0, color::TEXT);
    widgets.open.corner_radius = radius;
    widgets.open.expansion = 0.0;
}

/// egui's default fonts, preceded by Segoe UI when available, with Phosphor
/// icons and a Korean-capable fallback for game strings.
fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let system_font = |file: &str| {
        let windows = std::env::var_os("WINDIR").map(PathBuf::from)?;
        std::fs::read(windows.join("Fonts").join(file)).ok()
    };

    if let Some(bytes) = system_font("segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), Arc::new(FontData::from_owned(bytes)));
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "segoe".into());
    }

    // The bold family is the bold face followed by everything the regular family has.
    let mut bold_chain = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(bytes) = system_font("seguisb.ttf").or_else(|| system_font("segoeuib.ttf")) {
        fonts.font_data.insert("segoe-bold".into(), Arc::new(FontData::from_owned(bytes)));
        bold_chain.insert(0, "segoe-bold".into());
    }
    fonts.families.insert(bold_family(), bold_chain);

    // Without a CJK face, Hangul/Han from StringResource render as empty boxes.
    for (name, bytes) in cjk_fallback_fonts() {
        fonts.font_data.insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
        for family in fonts.families.values_mut() {
            family.push(name.clone());
        }
    }

    fonts
}

/// Platform fonts that cover game string glyphs. Windows uses Malgun Gothic;
/// macOS falls back to AppleGothic / Arial Unicode (egui cannot use `.ttc` collections here).
fn cjk_fallback_fonts() -> Vec<(String, Vec<u8>)> {
    let mut loaded = Vec::new();

    if let Some(windows) = std::env::var_os("WINDIR").map(PathBuf::from) {
        if let Ok(bytes) = std::fs::read(windows.join("Fonts").join("malgun.ttf")) {
            loaded.push(("malgun".into(), bytes));
        }
    }

    let mac_candidates: &[(&str, &Path)] = &[
        ("apple-gothic", Path::new("/System/Library/Fonts/Supplemental/AppleGothic.ttf")),
        ("arial-unicode", Path::new("/System/Library/Fonts/Supplemental/Arial Unicode.ttf")),
        ("arial-unicode", Path::new("/Library/Fonts/Arial Unicode.ttf")),
    ];
    for (name, path) in mac_candidates {
        if loaded.iter().any(|(existing, _)| existing == *name) {
            continue;
        }
        if let Ok(bytes) = std::fs::read(path) {
            loaded.push(((*name).into(), bytes));
        }
    }
    loaded
}

/// The window icon, decoded from the embedded `.ico`.
pub fn window_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(include_bytes!("../../../assets/Logo.ico")).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData { rgba: image.into_raw(), width, height })
}
