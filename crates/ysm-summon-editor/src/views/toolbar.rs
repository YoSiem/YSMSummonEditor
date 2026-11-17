//! Top bar: branding on the left, global actions on the right.

use eframe::egui::{self, Align, Color32, Layout, Margin, RichText, Ui, Vec2};

use super::ViewContext;
use crate::action::Action;
use crate::icons;
use crate::theme::{self, color};
use crate::widgets::{self, icon_text};

pub fn show(ui: &mut Ui, view: &mut ViewContext<'_>) {
    let frame = egui::Frame::new().fill(color::WINDOW_BG).inner_margin(Margin::symmetric(12, 8));
    egui::Panel::top("toolbar").frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            branding(ui, view);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| actions(ui, view));
        });
    });
}

fn branding(ui: &mut Ui, view: &ViewContext<'_>) {
    if let Some(logo) = view.textures.logo() {
        let height = 28.0;
        let size = Vec2::new(height * logo.aspect_ratio(), height);
        ui.add(egui::Image::from_texture((logo.id(), size)));
    }
    ui.label(RichText::new("Summon Editor").font(theme::bold(21.0)));
    ui.label(RichText::new("made by YoSiem").small().color(color::TEXT_MUTED));
    if view.state.demo {
        widgets::badge(ui, "DEMO DATA", color::WARNING, color::PANEL_ALT)
            .on_hover_text("Running on built-in sample data. Nothing is written to a database.");
    }
}

/// Laid out right to left, so the first widget here is the rightmost one.
fn actions(ui: &mut Ui, view: &mut ViewContext<'_>) {
    let state = view.state;
    let session = &state.session;

    if widgets::button(ui, icons::SETTINGS, "Settings").on_hover_text("Database connection and dump folder").clicked() {
        view.actions.push(Action::OpenSettings);
    }

    save_mode(ui, view);

    let stats = ui
        .add_enabled(
            state.can_open_stats(),
            egui::Button::new(icon_text(icons::STATS, "Stats")).min_size(Vec2::new(0.0, 30.0)),
        )
        .on_hover_text("Base stats, growth bonuses and the scaling calculator")
        .on_disabled_hover_text("Select a summon first");
    if stats.clicked() {
        view.actions.push(Action::OpenStats);
    }

    ui.separator();

    let changes = session.change_count();
    let discard = ui
        .add_enabled(
            changes > 0,
            egui::Button::new(RichText::new(icons::DISCARD).size(16.0)).min_size(Vec2::splat(30.0)),
        )
        .on_hover_text("Discard all staged changes")
        .on_disabled_hover_text("Nothing to discard");
    if discard.clicked() {
        view.actions.push(Action::AskDiscardAll);
    }

    let redo = ui
        .add_enabled(
            session.can_redo(),
            egui::Button::new(RichText::new(icons::REDO).size(16.0)).min_size(Vec2::splat(30.0)),
        )
        .on_hover_text("Redo (Ctrl+Y)");
    if redo.clicked() {
        view.actions.push(Action::Redo);
    }

    let undo = ui
        .add_enabled(
            session.can_undo(),
            egui::Button::new(RichText::new(icons::UNDO).size(16.0)).min_size(Vec2::splat(30.0)),
        )
        .on_hover_text("Undo (Ctrl+Z)");
    if undo.clicked() {
        view.actions.push(Action::Undo);
    }

    let label = if changes > 0 { format!("Save  ·  {changes}") } else { "Save".to_owned() };
    let mut save = egui::Button::new(RichText::new(icon_text(icons::SAVE, &label)).color(Color32::WHITE))
        .min_size(Vec2::new(104.0, 30.0));
    if changes > 0 {
        save = save.fill(color::PRIMARY);
    }
    let save = ui
        .add_enabled(changes > 0, save)
        .on_hover_text("Review and save the staged changes (Ctrl+S)")
        .on_disabled_hover_text("No staged changes");
    if save.clicked() {
        view.actions.push(Action::Save);
    }
}

/// Segmented control choosing where a save goes.
fn save_mode(ui: &mut Ui, view: &mut ViewContext<'_>) {
    let to_sql = view.state.settings.save_to_sql_mode;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let sql = ui
            .add(egui::Button::selectable(to_sql, icon_text(icons::SQL_FILE, ".sql")).min_size(Vec2::new(0.0, 30.0)))
            .on_hover_text("Save writes a .sql script; the database is not touched");
        if sql.clicked() && !to_sql {
            view.actions.push(Action::SetSaveToSql(true));
        }
        let db = ui
            .add(egui::Button::selectable(!to_sql, icon_text(icons::DATABASE, "DB")).min_size(Vec2::new(0.0, 30.0)))
            .on_hover_text("Save writes directly to the database");
        if db.clicked() && to_sql {
            view.actions.push(Action::SetSaveToSql(false));
        }
    });
    ui.label(widgets::muted("Save to"));
}
