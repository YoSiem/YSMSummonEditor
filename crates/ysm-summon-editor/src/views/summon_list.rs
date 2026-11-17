//! Left panel: searchable, virtualized list of summons.

use std::time::Instant;

use eframe::egui::{
    self, Align, Align2, CornerRadius, FontId, Key, Layout, Margin, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use ysm_core::model::SummonListItem;

use crate::action::Action;
use crate::icons;
use crate::state::{AppState, ConnectionStatus};
use crate::textures::TextureStore;
use crate::theme::{self, color};
use crate::widgets;

const ROW_HEIGHT: f32 = 46.0;

/// Takes the state mutably because the search box and filter toggles are
/// bound to it directly; everything else still goes through actions.
pub fn show(ui: &mut Ui, state: &mut AppState, textures: &mut TextureStore, actions: &mut Vec<Action>) {
    let frame =
        egui::Frame::new().fill(color::WINDOW_BG).inner_margin(Margin { left: 12, right: 4, top: 0, bottom: 8 });
    egui::Panel::left("summon-list").frame(frame).resizable(true).default_size(300.0).min_size(240.0).show(ui, |ui| {
        theme::card().show(ui, |ui| {
            ui.set_min_height(ui.available_height());
            search_row(ui, state, actions);
            ui.add_space(2.0);
            list(ui, state, textures, actions);
        });
    });
}

fn search_row(ui: &mut Ui, state: &mut AppState, actions: &mut Vec<Action>) {
    // Right to left: the filter button takes its natural width and the search box the rest.
    // (Sizing the box by hand makes the row wider than the panel, which then grows every frame.)
    ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), 32.0), Layout::right_to_left(Align::Center), |ui| {
        let filter_active = state.filter.include_riding_only || state.filter.final_only;
        let icon = RichText::new(icons::FILTER).size(16.0).color(if filter_active {
            color::ACCENT_BRIGHT
        } else {
            color::TEXT
        });
        ui.menu_button(icon, |ui| {
            ui.set_min_width(210.0);
            let mut changed = ui.checkbox(&mut state.filter.final_only, "Final evolutions only").changed();
            changed |= ui
                .checkbox(&mut state.filter.include_riding_only, format!("{}  Include riding-only", icons::MOUNT))
                .changed();
            if changed {
                actions.push(Action::Search);
            }
        })
        .response
        .on_hover_text("Filter the list");

        let search = ui.add(
            egui::TextEdit::singleline(&mut state.search_text)
                .hint_text(format!("{}  Search by ID, name or card", icons::SEARCH))
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(8, 6)),
        );
        if std::mem::take(&mut state.focus_search) {
            search.request_focus();
        }
        if search.changed() {
            state.search_edited_at = Some(Instant::now());
        }
        if search.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
            state.search_edited_at = None;
            actions.push(Action::Search);
        }
    });

    ui.horizontal(|ui| {
        if state.summons_loading {
            ui.spinner();
            ui.label(widgets::muted("Loading summons…"));
        } else {
            ui.label(widgets::muted(format!("{} summons", state.summons.len())).small());
        }
    });
}

fn list(ui: &mut Ui, state: &AppState, textures: &mut TextureStore, actions: &mut Vec<Action>) {
    if state.summons.is_empty() && !state.summons_loading {
        match &state.connection {
            ConnectionStatus::NotConfigured | ConnectionStatus::Failed(_) => {
                widgets::empty_state(ui, icons::NO_CONNECTION, "No database connection", "Set it up to load summons.");
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    if widgets::button(ui, icons::SETTINGS, "Open Settings").clicked() {
                        actions.push(Action::OpenSettings);
                    }
                });
            }
            ConnectionStatus::Connecting => widgets::empty_state(ui, icons::CONNECTION, "Connecting…", ""),
            _ => widgets::empty_state(ui, icons::SEARCH, "No summons found", "Try a different search."),
        }
        return;
    }

    ui.spacing_mut().item_spacing.y = 2.0;
    egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, ROW_HEIGHT, state.summons.len(), |ui, range| {
        for summon in &state.summons[range] {
            let selected = state.selected_summon == Some(summon.summon_id);
            if row(ui, summon, selected, textures).clicked() && !selected {
                actions.push(Action::SelectSummon(summon.clone()));
            }
        }
    });
}

fn row(ui: &mut Ui, summon: &SummonListItem, selected: bool, textures: &mut TextureStore) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let fill = if selected {
        Some(color::ROW_SELECTED)
    } else if response.hovered() {
        Some(color::ROW_HOVER)
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, CornerRadius::same(theme::RADIUS), fill);
    }

    let icon_rect = egui::Rect::from_min_size(
        rect.left_center() + Vec2::new(6.0, -theme::ICON_SIZE / 2.0),
        Vec2::splat(theme::ICON_SIZE),
    );
    let radius = CornerRadius::same(4);
    let texture = textures.icon(summon.face_file_name.as_deref());
    egui::Image::from_texture((texture, icon_rect.size())).corner_radius(radius).paint_at(ui, icon_rect);
    ui.painter().rect_stroke(icon_rect, radius, Stroke::new(1.0, color::CONTROL_BORDER), StrokeKind::Inside);

    let text_left = icon_rect.right() + 10.0;
    let painter = ui.painter().with_clip_rect(rect.shrink2(Vec2::new(6.0, 0.0)));
    painter.text(
        egui::pos2(text_left, rect.center().y - 9.0),
        Align2::LEFT_CENTER,
        &summon.summon_name,
        theme::bold(13.5),
        color::TEXT,
    );
    let mut detail = summon.summon_id.to_string();
    if summon.is_riding_only {
        detail.push_str(&format!("   {}", icons::MOUNT));
    }
    painter.text(
        egui::pos2(text_left, rect.center().y + 9.0),
        Align2::LEFT_CENTER,
        detail,
        FontId::proportional(12.0),
        if selected { color::TEXT } else { color::TEXT_MUTED },
    );

    response.on_hover_ui(|ui| {
        ui.label(RichText::new(&summon.summon_name).font(theme::bold(14.0)));
        ui.label(widgets::muted(format!(
            "ID {}  ·  Form {}  ·  Evolves into {}",
            summon.summon_id,
            summon.form,
            if summon.evolve_target == 0 { "—".to_owned() } else { summon.evolve_target.to_string() }
        )));
        ui.label(widgets::muted(format!(
            "{}  {} ({})",
            icons::CARD,
            summon.card_name.as_deref().unwrap_or("Unknown card"),
            summon.card_id
        )));
    })
}
