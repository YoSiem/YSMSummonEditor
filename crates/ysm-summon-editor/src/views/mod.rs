//! The panels of the main window. Each view reads [`AppState`] and pushes
//! [`Action`]s; none of them changes application state directly.

pub mod effective_panel;
pub mod status_bar;
pub mod summon_header;
pub mod summon_list;
pub mod toolbar;
pub mod tree_panel;

use eframe::egui::{self, Color32, Response, RichText, Ui};

use crate::action::Action;
use crate::icons;
use crate::state::AppState;
use crate::textures::TextureStore;
use crate::theme::color;

/// What every view needs to draw itself.
pub struct ViewContext<'a> {
    pub state: &'a AppState,
    pub textures: &'a mut TextureStore,
    pub actions: &'a mut Vec<Action>,
}

/// A tab header with an optional "has staged changes" dot.
pub(crate) fn tab(ui: &mut Ui, selected: bool, enabled: bool, title: &str, subtitle: &str, dirty: bool) -> Response {
    let title_color = if enabled { color::TEXT } else { color::TEXT_FAINT };
    let mut job = egui::text::LayoutJob::default();
    let body = egui::TextStyle::Button.resolve(ui.style());
    let small = egui::TextStyle::Small.resolve(ui.style());
    let append = |job: &mut egui::text::LayoutJob, text: &str, font: egui::FontId, tint: Color32| {
        job.append(text, 0.0, egui::TextFormat { font_id: font, color: tint, ..Default::default() });
    };
    append(&mut job, title, body, if selected { Color32::WHITE } else { title_color });
    if !subtitle.is_empty() {
        append(&mut job, &format!("  {subtitle}"), small.clone(), color::TEXT_MUTED);
    }
    if dirty {
        append(&mut job, &format!("  {}", icons::DOT), small, color::DIRTY_MARK);
    }
    ui.add(egui::Button::selectable(selected, job).min_size(egui::Vec2::new(0.0, 30.0)))
}

/// Dimmed text for zero values, so the meaningful numbers of a grid stand out.
pub(crate) fn value_color(value: i32, changed: bool) -> Color32 {
    if changed {
        color::DIRTY_MARK
    } else if value == 0 {
        color::TEXT_FAINT
    } else {
        color::TEXT
    }
}

/// A grid column header with an explanatory tooltip.
pub(crate) fn header_cell(ui: &mut Ui, text: &str, tooltip: &str) {
    ui.centered_and_justified(|ui| {
        let label = ui.add(egui::Label::new(RichText::new(text).color(color::TEXT_MUTED).small()).selectable(false));
        if !tooltip.is_empty() {
            label.on_hover_text(tooltip);
        }
    });
}
