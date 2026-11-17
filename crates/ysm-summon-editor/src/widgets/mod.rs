//! Small reusable widgets shared by views and dialogs.

pub mod game_tooltip;
pub mod int_cell;
pub mod modal;
pub mod toast;

use eframe::egui::{self, Color32, CornerRadius, Response, RichText, Sense, Stroke, StrokeKind, TextureId, Ui, Vec2};

use crate::theme::{self, color};

/// `icon` followed by `text`, for buttons and menu items.
pub fn icon_text(icon: &str, text: &str) -> String {
    format!("{icon}  {text}")
}

/// The main action of a dialog or toolbar.
pub fn primary_button(ui: &mut Ui, icon: &str, text: &str) -> Response {
    filled_button(ui, icon, text, color::PRIMARY)
}

/// A destructive action.
pub fn danger_button(ui: &mut Ui, icon: &str, text: &str) -> Response {
    filled_button(ui, icon, text, color::DANGER_BG)
}

fn filled_button(ui: &mut Ui, icon: &str, text: &str, fill: Color32) -> Response {
    let label = RichText::new(icon_text(icon, text)).color(Color32::WHITE);
    ui.add(egui::Button::new(label).fill(fill).min_size(Vec2::new(96.0, 30.0)))
}

pub fn button(ui: &mut Ui, icon: &str, text: &str) -> Response {
    ui.add(egui::Button::new(icon_text(icon, text)).min_size(Vec2::new(0.0, 30.0)))
}

/// A square button showing only an icon, with a tooltip.
pub fn icon_button(ui: &mut Ui, icon: &str, tooltip: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(icon).size(16.0)).min_size(Vec2::splat(30.0))).on_hover_text(tooltip)
}

/// A game icon with the thin border every icon in the editor has.
pub fn game_icon(ui: &mut Ui, texture: TextureId, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(4);
        egui::Image::from_texture((texture, Vec2::splat(size))).corner_radius(radius).paint_at(ui, rect);
        ui.painter().rect_stroke(rect, radius, Stroke::new(1.0, color::CONTROL_BORDER), StrokeKind::Inside);
    }
    response
}

/// A small rounded label, e.g. `RND[301]`.
pub fn badge(ui: &mut Ui, text: &str, foreground: Color32, background: Color32) -> Response {
    egui::Frame::new()
        .fill(background)
        .stroke(Stroke::new(1.0, foreground.gamma_multiply(0.6)))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(RichText::new(text).small().color(foreground).font(theme::bold(11.5)))
                    .selectable(false),
            )
        })
        .response
}

/// Muted secondary text.
pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text).color(color::TEXT_MUTED)
}

/// A centred message for empty lists and panels.
pub fn empty_state(ui: &mut Ui, icon: &str, title: &str, hint: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() * 0.25).clamp(12.0, 120.0));
        ui.label(RichText::new(icon).size(40.0).color(color::TEXT_FAINT));
        ui.add_space(4.0);
        ui.label(RichText::new(title).font(theme::bold(15.0)).color(color::TEXT_MUTED));
        if !hint.is_empty() {
            ui.label(RichText::new(hint).color(color::TEXT_FAINT));
        }
    });
}

/// Paints a row background behind a table cell. Call first in every cell of the row.
pub fn paint_row_background(ui: &Ui, fill: Color32) {
    // Widened past the cell so neighbouring cells join into one band; the
    // painter is re-clipped because a cell's own clip rect stops at its edge.
    let rect = ui.max_rect().expand2(Vec2::new(ui.spacing().item_spacing.x, 0.0));
    ui.painter().with_clip_rect(rect.intersect(ui.ctx().content_rect())).rect_filled(rect, 0.0, fill);
}
