//! Review of everything a save will change, with the equivalent SQL script.

use eframe::egui::{self, Align, Color32, Layout, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use ysm_core::review::{ReviewKind, ReviewRow};

use super::DialogContext;
use crate::action::Action;
use crate::icons;
use crate::theme::color;
use crate::widgets::toast::ToastKind;
use crate::widgets::{self, modal};

const CONTENT_HEIGHT: f32 = 440.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Changes,
    Sql,
}

pub struct SaveReviewDialog {
    rows: Vec<ReviewRow>,
    sql: String,
    /// Whether confirming exports a file instead of writing to the database.
    to_sql_file: bool,
    /// Where a database save would go, for the footer.
    target: String,
    tab: Tab,
}

impl SaveReviewDialog {
    pub fn new(rows: Vec<ReviewRow>, sql: String, to_sql_file: bool, target: String) -> Self {
        Self { rows, sql, to_sql_file, target, tab: Tab::Changes }
    }

    pub fn show(&mut self, ctx: &egui::Context, id: egui::Id, env: &mut DialogContext<'_>) -> bool {
        let mut confirmed = false;
        let mut cancelled = false;

        let outcome = modal::show(ctx, id, icons::CHANGES, color::ACCENT_BRIGHT, "Review Changes", 980.0, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let changes = widgets::icon_text(icons::CHANGES, &format!("Changes  {}", self.rows.len()));
                if ui.add(egui::Button::selectable(self.tab == Tab::Changes, changes)).clicked() {
                    self.tab = Tab::Changes;
                }
                if ui
                    .add(egui::Button::selectable(self.tab == Tab::Sql, widgets::icon_text(icons::CODE, "SQL script")))
                    .clicked()
                {
                    self.tab = Tab::Sql;
                }
                if self.tab == Tab::Sql {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::button(ui, icons::CLIPBOARD, "Copy").clicked() {
                            env.actions.push(Action::CopyText(self.sql.clone()));
                            env.actions
                                .push(Action::Toast(ToastKind::Info, "SQL script copied to the clipboard.".into()));
                        }
                    });
                }
            });
            ui.add_space(4.0);

            match self.tab {
                Tab::Changes => self.changes(ui),
                Tab::Sql => self.script(ui),
            }

            modal::footer(ui, |ui| {
                if widgets::button(ui, icons::CLOSE, "Cancel").clicked() {
                    cancelled = true;
                }
                let (icon, label) = if self.to_sql_file {
                    (icons::SQL_FILE, "Export .sql…")
                } else {
                    (icons::DATABASE, "Save to database")
                };
                if widgets::primary_button(ui, icon, label).clicked() {
                    confirmed = true;
                }
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if self.to_sql_file {
                        ui.label(widgets::muted("Mode .sql: a script is written; the database is not touched."));
                    } else {
                        ui.label(widgets::muted(format!("Target: {}", self.target)));
                    }
                });
            });
        });

        if confirmed {
            env.actions.push(Action::ReviewConfirmed);
            return false;
        }
        if cancelled || outcome.dismissed {
            env.actions.push(Action::SaveCancelled);
            return false;
        }
        true
    }

    fn changes(&self, ui: &mut Ui) {
        TableBuilder::new(ui)
            .id_salt("review-changes")
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .auto_shrink(false)
            .min_scrolled_height(CONTENT_HEIGHT)
            .max_scroll_height(CONTENT_HEIGHT)
            .column(Column::exact(30.0))
            .column(Column::exact(110.0))
            .column(Column::initial(280.0).at_least(160.0).clip(true).resizable(true))
            .column(Column::exact(130.0).clip(true))
            .column(Column::initial(170.0).at_least(80.0).clip(true).resizable(true))
            .column(Column::remainder().clip(true))
            .header(26.0, |mut header| {
                for title in ["", "Type", "Target", "Field", "Before", "After"] {
                    header.col(|ui| {
                        ui.label(RichText::new(title).small().color(color::TEXT_MUTED));
                    });
                }
            })
            .body(|body| {
                body.rows(28.0, self.rows.len(), |mut table_row| {
                    let row = &self.rows[table_row.index()];
                    let (icon, tint, description) = kind_style(row.kind);
                    table_row.col(|ui| {
                        ui.label(RichText::new(icon).color(tint)).on_hover_text(description);
                    });
                    table_row.col(|ui| {
                        ui.label(row.category);
                    });
                    table_row.col(|ui| {
                        ui.add(egui::Label::new(row.target.as_str()).truncate()).on_hover_text(&row.target);
                    });
                    table_row.col(|ui| {
                        ui.label(widgets::muted(row.field.as_str()));
                    });
                    table_row.col(|ui| {
                        let text = RichText::new(&row.before).color(color::TEXT_MUTED);
                        let text = if row.kind == ReviewKind::Delete { text.strikethrough() } else { text };
                        ui.add(egui::Label::new(text).truncate()).on_hover_text(&row.before);
                    });
                    table_row.col(|ui| {
                        ui.add(egui::Label::new(RichText::new(&row.after).color(tint)).truncate())
                            .on_hover_text(&row.after);
                    });
                });
            });
    }

    fn script(&self, ui: &mut Ui) {
        egui::Frame::new().fill(color::INPUT_BG).corner_radius(6).inner_margin(8).show(ui, |ui| {
            egui::ScrollArea::both()
                .id_salt("review-sql")
                .auto_shrink(false)
                .max_height(CONTENT_HEIGHT)
                .min_scrolled_height(CONTENT_HEIGHT)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(&self.sql).monospace()).extend());
                });
        });
    }
}

fn kind_style(kind: ReviewKind) -> (&'static str, Color32, &'static str) {
    match kind {
        ReviewKind::Insert => (icons::INSERTED, color::SUCCESS, "Inserted"),
        ReviewKind::Update => (icons::EDIT, color::WARNING, "Updated"),
        ReviewKind::Delete => (icons::DELETED, color::DANGER, "Deleted"),
    }
}
