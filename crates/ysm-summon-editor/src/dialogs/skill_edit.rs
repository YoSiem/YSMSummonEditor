//! Editor for the columns of one `SkillResource` row.

use eframe::egui::{self, Align, Layout, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use ysm_core::commit::{FieldChange, SkillUpdate};
use ysm_core::value::{BUSINESS_FIELDS, FieldKind, FieldValue, SkillRecord, is_business_field};

use super::DialogContext;
use crate::action::Action;
use crate::icons;
use crate::theme::{self, color};
use crate::widgets::{self, modal, paint_row_background};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Business,
    All,
}

struct FieldRow {
    name: String,
    kind: FieldKind,
    db_value: FieldValue,
    /// The database value as text; a row is dirty when `text` differs from it.
    db_text: String,
    text: String,
    read_only: bool,
    business: bool,
    error: Option<String>,
}

impl FieldRow {
    fn is_dirty(&self) -> bool {
        self.text != self.db_text
    }

    fn validate(&mut self) {
        self.error =
            if self.is_dirty() { FieldValue::parse(self.kind, &self.text).err().map(|e| e.to_string()) } else { None };
    }
}

pub struct SkillEditDialog {
    skill_id: i32,
    skill_name: String,
    rows: Vec<FieldRow>,
    tab: Tab,
    filter: String,
    only_changed: bool,
}

impl SkillEditDialog {
    /// `pending` is the update already staged for this skill, if any; its
    /// values are shown as the current ones so reopening never loses edits.
    pub fn new(record: SkillRecord, pending: Option<&SkillUpdate>, skill_name: String) -> Self {
        let mut rows: Vec<FieldRow> = record
            .fields
            .into_iter()
            .map(|field| {
                let db_text = field.value.to_string();
                let staged =
                    pending.and_then(|update| update.changes.iter().find(|c| c.name.eq_ignore_ascii_case(&field.name)));
                FieldRow {
                    read_only: field.is_read_only(),
                    business: is_business_field(&field.name),
                    text: staged.map_or_else(|| db_text.clone(), |change| change.new.to_string()),
                    db_text,
                    name: field.name,
                    kind: field.kind,
                    db_value: field.value,
                    error: None,
                }
            })
            .collect();
        rows.iter_mut().for_each(FieldRow::validate);

        Self {
            skill_id: record.skill_id,
            skill_name,
            rows,
            tab: Tab::Business,
            filter: String::new(),
            only_changed: false,
        }
    }

    fn changed_count(&self) -> usize {
        self.rows.iter().filter(|row| row.is_dirty()).count()
    }

    fn has_errors(&self) -> bool {
        self.rows.iter().any(|row| row.error.is_some())
    }

    fn build_update(&self) -> SkillUpdate {
        let changes = self
            .rows
            .iter()
            .filter(|row| row.is_dirty() && !row.read_only)
            .filter_map(|row| {
                let new = FieldValue::parse(row.kind, &row.text).ok()?;
                Some(FieldChange { name: row.name.clone(), kind: row.kind, old: row.db_value.clone(), new })
            })
            .collect();
        SkillUpdate { skill_id: self.skill_id, changes }
    }

    /// Indices of the rows shown on the current tab, in display order.
    fn visible_rows(&self) -> Vec<usize> {
        let filter = self.filter.trim().to_lowercase();
        let passes = |row: &FieldRow| {
            (filter.is_empty() || row.name.to_lowercase().contains(&filter)) && (!self.only_changed || row.is_dirty())
        };
        match self.tab {
            Tab::All => (0..self.rows.len()).filter(|i| passes(&self.rows[*i])).collect(),
            // Business fields keep their curated order rather than the table's column order.
            Tab::Business => BUSINESS_FIELDS
                .iter()
                .filter_map(|name| self.rows.iter().position(|row| row.name.eq_ignore_ascii_case(name)))
                .filter(|i| passes(&self.rows[*i]))
                .collect(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, id: egui::Id, env: &mut DialogContext<'_>) -> bool {
        let title = format!("Skill {} · {}", self.skill_id, self.skill_name);
        let mut apply = false;
        let mut cancelled = false;

        let outcome = modal::show(ctx, id, icons::EDIT, color::ACCENT_BRIGHT, &title, 760.0, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let business = self.rows.iter().filter(|row| row.business).count();
                if ui
                    .add(egui::Button::selectable(self.tab == Tab::Business, format!("Business fields  {business}")))
                    .clicked()
                {
                    self.tab = Tab::Business;
                }
                if ui
                    .add(egui::Button::selectable(self.tab == Tab::All, format!("All columns  {}", self.rows.len())))
                    .clicked()
                {
                    self.tab = Tab::All;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.checkbox(&mut self.only_changed, "Changed only");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.filter)
                            .hint_text(format!("{}  Filter columns", icons::SEARCH))
                            .desired_width(200.0),
                    );
                });
            });
            ui.add_space(4.0);

            let visible = self.visible_rows();
            if visible.is_empty() {
                ui.allocate_ui([ui.available_width(), 420.0].into(), |ui| {
                    widgets::empty_state(
                        ui,
                        icons::SEARCH,
                        "No columns match",
                        "Clear the filter to see every column.",
                    );
                });
            } else {
                self.grid(ui, &visible);
            }

            modal::footer(ui, |ui| {
                if widgets::button(ui, icons::CLOSE, "Cancel").clicked() {
                    cancelled = true;
                }
                ui.add_enabled_ui(!self.has_errors(), |ui| {
                    if widgets::primary_button(ui, icons::CHECK, "Apply").clicked() {
                        apply = true;
                    }
                });
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if self.has_errors() {
                        modal::note(ui, icons::WARNING, "Fix the highlighted values before applying.", color::WARNING);
                    } else {
                        let changed = self.changed_count();
                        let text =
                            if changed == 0 { "No changes".to_owned() } else { format!("{changed} column(s) changed") };
                        ui.label(widgets::muted(text));
                    }
                });
            });
        });

        if apply {
            env.actions.push(Action::StageSkillUpdate(self.build_update()));
            return false;
        }
        let escaped = outcome.dismissed && ctx.input(|input| input.key_pressed(egui::Key::Escape));
        !(cancelled || escaped)
    }

    fn grid(&mut self, ui: &mut Ui, visible: &[usize]) {
        let rows = &mut self.rows;
        TableBuilder::new(ui)
            .id_salt(("skill-fields", self.tab == Tab::All))
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .auto_shrink(false)
            .min_scrolled_height(420.0)
            .max_scroll_height(420.0)
            .column(Column::exact(210.0).clip(true))
            .column(Column::exact(70.0))
            .column(Column::remainder().at_least(160.0))
            .column(Column::exact(150.0).clip(true))
            .column(Column::exact(34.0))
            .header(26.0, |mut header| {
                for title in ["Column", "Type", "Value", "In database", ""] {
                    header.col(|ui| {
                        ui.label(RichText::new(title).small().color(color::TEXT_MUTED));
                    });
                }
            })
            .body(|body| {
                body.rows(32.0, visible.len(), |mut table_row| {
                    let row = &mut rows[visible[table_row.index()]];
                    let dirty = row.is_dirty();
                    let fill = dirty.then_some(color::ROW_DIRTY);
                    let background = |ui: &Ui| {
                        if let Some(fill) = fill {
                            paint_row_background(ui, fill);
                        }
                    };

                    table_row.col(|ui| {
                        background(ui);
                        let name = RichText::new(&row.name);
                        ui.label(if row.business { name.font(theme::bold(13.5)) } else { name });
                    });
                    table_row.col(|ui| {
                        background(ui);
                        ui.label(widgets::muted(row.kind.label()).small());
                    });
                    table_row.col(|ui| {
                        background(ui);
                        value_editor(ui, row);
                    });
                    table_row.col(|ui| {
                        background(ui);
                        let original = if row.db_value.is_null() { "NULL".to_owned() } else { row.db_text.clone() };
                        ui.label(widgets::muted(original)).on_hover_text(&row.db_text);
                    });
                    table_row.col(|ui| {
                        background(ui);
                        if dirty
                            && ui
                                .add(egui::Button::new(icons::DISCARD).frame(false))
                                .on_hover_text("Restore the database value")
                                .clicked()
                        {
                            row.text = row.db_text.clone();
                            row.error = None;
                        }
                    });
                });
            });
    }
}

fn value_editor(ui: &mut Ui, row: &mut FieldRow) {
    if row.read_only {
        ui.label(widgets::muted(row.text.as_str())).on_hover_text("This column cannot be edited.");
        return;
    }

    if row.kind == FieldKind::Bool {
        let mut checked = matches!(row.text.trim().to_ascii_lowercase().as_str(), "1" | "true");
        if ui.checkbox(&mut checked, "").changed() {
            row.text = if checked { "1" } else { "0" }.to_owned();
            row.validate();
        }
        return;
    }

    let mut edit = egui::TextEdit::singleline(&mut row.text).desired_width(f32::INFINITY);
    if row.error.is_some() {
        edit = edit.text_color(color::DANGER);
    }
    let response = ui.add(edit);
    if response.changed() {
        row.validate();
    }
    if let Some(error) = &row.error {
        response.on_hover_text(RichText::new(error).color(color::DANGER));
    }
}
