//! Common chrome for modal dialogs.

use eframe::egui::{self, Color32, RichText, Ui};

use crate::theme::{self, color};

pub struct ModalOutcome {
    /// The user pressed Escape or clicked outside the dialog.
    pub dismissed: bool,
}

/// Shows a modal with an icon, a title and a fixed content width.
pub fn show(
    ctx: &egui::Context,
    id: egui::Id,
    icon: &str,
    icon_color: Color32,
    title: &str,
    width: f32,
    add_contents: impl FnOnce(&mut Ui),
) -> ModalOutcome {
    let frame = egui::Frame::popup(&ctx.global_style())
        .fill(color::PANEL_BG)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(18, 14));
    let response = egui::Modal::new(id).frame(frame).show(ctx, |ui| {
        ui.set_width(width.min(ui.ctx().content_rect().width() - 100.0));
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).size(22.0).color(icon_color));
            ui.label(RichText::new(title).font(theme::bold(18.0)));
        });
        ui.add_space(2.0);
        ui.separator();
        ui.add_space(4.0);
        add_contents(ui);
    });
    ModalOutcome { dismissed: response.should_close() }
}

/// A right-aligned row of dialog buttons, separated from the content above.
pub fn footer<R>(ui: &mut Ui, add_buttons: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(2.0);
    // A bounded row: an unbounded right-to-left layout would claim all remaining height.
    let size = egui::Vec2::new(ui.available_width(), 32.0);
    ui.allocate_ui_with_layout(size, egui::Layout::right_to_left(egui::Align::Center), add_buttons).inner
}

/// A one-line status or validation message inside a dialog.
pub fn note(ui: &mut Ui, icon: &str, text: &str, tint: Color32) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(icon).color(tint));
        ui.label(RichText::new(text).color(tint));
    });
}

/// Section caption inside a dialog.
pub fn caption(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(theme::bold(13.5)).color(color::TEXT_MUTED));
}
