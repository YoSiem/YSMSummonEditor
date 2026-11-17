//! Right panel: skills of the active evolution merged across its trees.

use eframe::egui::{self, Align, Layout, Margin, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};
use ysm_core::session::EffectiveRow;

use super::{ViewContext, header_cell, value_color};
use crate::action::Action;
use crate::icons;
use crate::theme::{self, color};
use crate::widgets::int_cell::int_cell;
use crate::widgets::{self, game_tooltip, icon_text, paint_row_background};

pub fn show(ui: &mut Ui, view: &mut ViewContext<'_>) {
    let frame = egui::Frame::new().fill(color::WINDOW_BG).inner_margin(Margin { left: 4, right: 0, top: 0, bottom: 0 });
    egui::Panel::right("effective-skills").frame(frame).resizable(true).default_size(340.0).min_size(300.0).show(
        ui,
        |ui| {
            theme::card().show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                let stage = view.state.active_stage;
                let rows = view.state.session.effective_rows(stage);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon_text(icons::EFFECTIVE, "Effective Skills")).font(theme::bold(16.0)))
                        .on_hover_text("Rows of all five trees merged by Skill ID + Lv for the selected evolution.");
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(widgets::muted(format!("Evolution {stage} · {} skills", rows.len())).small());
                    });
                });
                ui.separator();

                if rows.is_empty() {
                    widgets::empty_state(
                        ui,
                        icons::EFFECTIVE,
                        "No skills",
                        "This evolution has no skills in its trees.",
                    );
                } else {
                    grid(ui, view, stage, &rows);
                }
            });
        },
    );
}

fn grid(ui: &mut Ui, view: &mut ViewContext<'_>, stage: u8, rows: &[EffectiveRow]) {
    let text_height = ui.text_style_height(&egui::TextStyle::Body);
    TableBuilder::new(ui)
        .id_salt(("effective-grid", stage))
        .sense(Sense::click())
        .striped(true)
        .cell_layout(Layout::left_to_right(Align::Center))
        .auto_shrink(false)
        .min_scrolled_height(0.0)
        .column(Column::exact(theme::ICON_SIZE + 6.0))
        .column(Column::exact(58.0))
        .column(Column::remainder().at_least(120.0).clip(true))
        .column(Column::exact(44.0))
        .column(Column::exact(44.0))
        .column(Column::exact(36.0))
        .header(text_height + 10.0, |mut header| {
            header.col(|_| {});
            header.col(|ui| header_cell(ui, "ID", "Skill ID"));
            header.col(|ui| header_cell(ui, "Skill", "Skill name; double-click to edit the skill"));
            header.col(|ui| header_cell(ui, "Min", "Lowest minimum skill level; editing applies to every merged row"));
            header.col(|ui| header_cell(ui, "Max", "Highest maximum skill level; editing applies to every merged row"));
            header.col(|ui| header_cell(ui, "Lv", "Required summon level"));
        })
        .body(|body| {
            body.rows(theme::ROW_HEIGHT, rows.len(), |mut table_row| {
                let row = &rows[table_row.index()];
                grid_row(&mut table_row, view, stage, row);
            });
        });
}

fn grid_row(table_row: &mut egui_extras::TableRow<'_, '_>, view: &mut ViewContext<'_>, stage: u8, row: &EffectiveRow) {
    let session = &view.state.session;
    let key = (row.skill_id, row.lv);
    let selected = view.state.selected_effective == Some(key);
    let dirty_fill = (row.is_dirty && !selected).then_some(color::ROW_DIRTY);
    let display = session.skill_display(row.skill_id);
    table_row.set_selected(selected);

    let menu = |ui: &mut Ui, actions: &mut Vec<Action>| {
        ui.set_min_width(190.0);
        if ui.button(icon_text(icons::EDIT, "Edit Skill")).clicked() {
            actions.push(Action::EditSkill(row.skill_id));
            ui.close();
        }
        if ui.button(icon_text(icons::DELETE, "Delete From Tree")).clicked() {
            actions.push(Action::AskDeleteEffective { stage, skill_id: row.skill_id, lv: row.lv });
            ui.close();
        }
    };
    let select = |response: &egui::Response, actions: &mut Vec<Action>| {
        if (response.clicked() || response.secondary_clicked() || response.gained_focus()) && !selected {
            actions.push(Action::SelectEffective(Some(key)));
        }
    };
    let background = |ui: &Ui| {
        if let Some(fill) = dirty_fill {
            paint_row_background(ui, fill);
        }
    };

    table_row.col(|ui| {
        background(ui);
        let icon = view.textures.icon(display.and_then(|d| d.icon_file_name.as_deref()));
        widgets::game_icon(ui, icon, theme::ICON_SIZE).on_hover_ui(|ui| game_tooltip::skill(ui, row.skill_id, display));
    });

    table_row.col(|ui| {
        background(ui);
        ui.centered_and_justified(|ui| {
            ui.add(egui::Label::new(row.skill_id.to_string()).selectable(false));
        });
    });

    let (_, name_response) = table_row.col(|ui| {
        background(ui);
        let tint = if display.is_some() { color::TEXT } else { color::WARNING };
        ui.add(
            egui::Label::new(RichText::new(session.skill_name(row.skill_id)).color(tint)).truncate().selectable(false),
        )
        .on_hover_ui(|ui| game_tooltip::skill(ui, row.skill_id, display));
        if row.linked > 1 {
            ui.label(widgets::muted(format!("×{}", row.linked)).small())
                .on_hover_text(format!("Merged from {} tree rows", row.linked));
        }
        for group_id in &row.group_ids {
            widgets::badge(ui, &format!("RND[{group_id}]"), color::ACCENT_BRIGHT, color::PANEL_ALT)
                .on_hover_ui(|ui| game_tooltip::random_group(ui, session, *group_id));
        }
    });
    if name_response.double_clicked() {
        view.actions.push(Action::EditSkill(row.skill_id));
    }

    for (is_min, value) in [(true, row.min_skill_lv), (false, row.max_skill_lv)] {
        table_row.col(|ui| {
            background(ui);
            let id = egui::Id::new(("effective-cell", stage, key, is_min));
            let output = int_cell(ui, id, value, value_color(value, false));
            select(&output.response, view.actions);
            if let Some(new_value) = output.committed {
                let (min, max) = if is_min { (new_value, row.max_skill_lv) } else { (row.min_skill_lv, new_value) };
                view.actions.push(Action::SetEffectiveLevels { stage, skill_id: row.skill_id, lv: row.lv, min, max });
            }
            output.response.context_menu(|ui| menu(ui, view.actions));
        });
    }

    table_row.col(|ui| {
        background(ui);
        ui.centered_and_justified(|ui| {
            ui.add(
                egui::Label::new(RichText::new(row.lv.to_string()).color(value_color(row.lv, false))).selectable(false),
            );
        });
    });

    let response = table_row.response();
    select(&response, view.actions);
    response.context_menu(|ui| menu(ui, view.actions));
}
