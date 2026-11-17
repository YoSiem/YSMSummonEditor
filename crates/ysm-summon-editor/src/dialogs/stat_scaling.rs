//! Base stats, growth bonuses and the live scaling calculator of a summon.

use std::collections::BTreeMap;

use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke, Ui, Vec2};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use ysm_core::model::{CreatureEnhance, STAT_COUNT, Stat, StatScalingSnapshot};
use ysm_core::scaling::{self, MAX_ENHANCE_STAGE, MAX_LEVEL, MIN_LEVEL, ScalingInput, ScalingResult};
use ysm_core::scaling_edit::{ScalingDraft, round_bonus};

use super::DialogContext;
use crate::action::Action;
use crate::icons;
use crate::theme::{self, color};
use crate::widgets::{self, modal};

/// One colour per stat, used for the growth chart and its legend.
const STAT_COLORS: [Color32; STAT_COUNT] = [
    Color32::from_rgb(0xEB, 0x6B, 0x6B),
    Color32::from_rgb(0xF2, 0x99, 0x4A),
    Color32::from_rgb(0xF2, 0xC9, 0x4C),
    Color32::from_rgb(0x6F, 0xCF, 0x97),
    Color32::from_rgb(0x56, 0xCC, 0xF2),
    Color32::from_rgb(0xBB, 0x8F, 0xEB),
    Color32::from_rgb(0xD4, 0xA5, 0xC9),
];

/// Horizontal space a section's frame adds around its content (margins and border).
const SECTION_CHROME: f32 = 22.0;
const SECTION_HEIGHT: f32 = 452.0;

/// A titled card of fixed width. The content is laid out top to bottom
/// regardless of the layout the card itself sits in.
fn section(ui: &mut Ui, width: f32, add_contents: impl FnOnce(&mut Ui)) {
    theme::card().fill(color::PANEL_ALT).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.set_min_height(SECTION_HEIGHT);
            add_contents(ui);
        });
    });
}

pub struct StatScalingDialog {
    summon_name: String,
    draft: ScalingDraft,
    amplify: BTreeMap<i32, Decimal>,
    level: i32,
    enhance_stage: i32,
    /// Final stat values per level for the chart, recomputed when its inputs change.
    curve: Vec<[i32; STAT_COUNT]>,
    curve_inputs: Option<(StatScalingSnapshot, i32)>,
}

impl StatScalingDialog {
    /// `db` is the database state, `staged` the same with edits staged in the session applied.
    pub fn new(
        summon_name: String,
        db: &StatScalingSnapshot,
        staged: &StatScalingSnapshot,
        creature_enhance: &[CreatureEnhance],
    ) -> Self {
        Self {
            summon_name,
            draft: ScalingDraft::new(db, staged),
            // Later rows win when an enhance level is listed twice.
            amplify: creature_enhance.iter().map(|e| (e.enhance_level, e.stat_amplify)).collect(),
            level: MIN_LEVEL,
            enhance_stage: 0,
            curve: Vec::new(),
            curve_inputs: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, id: egui::Id, env: &mut DialogContext<'_>) -> bool {
        let title = format!("Stat Scaling · {}", self.summon_name);
        let mut apply = false;
        let mut cancelled = false;

        let outcome = modal::show(ctx, id, icons::STATS, color::ACCENT_BRIGHT, &title, 1180.0, |ui| {
            ui.label(widgets::muted(format!(
                "Summon {}  ·  Stat ID {}  ·  Evolution depth {}",
                self.draft.summon_id(),
                self.draft.stat_id(),
                self.draft.growth_depth()
            )));
            ui.add_space(4.0);

            let snapshot = self.draft.snapshot();
            let result = scaling::calculate(&ScalingInput {
                level: self.level,
                enhance_stage: self.enhance_stage,
                enhance_amplify: &self.amplify,
                snapshot: &snapshot,
            });
            if self.curve_inputs.as_ref() != Some(&(snapshot.clone(), self.enhance_stage)) {
                self.curve = scaling::growth_curve(&snapshot, &self.amplify, self.enhance_stage);
                self.curve_inputs = Some((snapshot, self.enhance_stage));
            }

            // Fixed widths for the two editors; the calculator takes whatever is left.
            let base_width = 150.0;
            let growth_width = 60.0 + 100.0 * self.draft.bonuses.len() as f32;
            let gaps = 2.0 * ui.spacing().item_spacing.x + 3.0 * SECTION_CHROME;
            let calculator_width = (ui.available_width() - base_width - growth_width - gaps).max(420.0);
            ui.horizontal_top(|ui| {
                section(ui, base_width, |ui| self.base_stats(ui));
                section(ui, growth_width, |ui| self.growth_bonuses(ui));
                section(ui, calculator_width, |ui| self.calculator(ui, &result));
            });

            modal::footer(ui, |ui| {
                if widgets::button(ui, icons::CLOSE, "Cancel").clicked() {
                    cancelled = true;
                }
                if widgets::primary_button(ui, icons::CHECK, "Apply").clicked() {
                    apply = true;
                }
                if ui
                    .add_enabled(
                        self.draft.is_dirty(),
                        egui::Button::new(widgets::icon_text(icons::DISCARD, "Reset to database")),
                    )
                    .clicked()
                {
                    self.draft.reset();
                }
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let text = if self.draft.is_dirty() { "Values differ from the database." } else { "No changes." };
                    ui.label(widgets::muted(text));
                });
            });
        });

        if apply {
            env.actions.push(Action::ApplyScaling(self.draft.build_result()));
            return false;
        }
        let escaped = outcome.dismissed && ctx.input(|input| input.key_pressed(egui::Key::Escape));
        !(cancelled || escaped)
    }

    fn base_stats(&mut self, ui: &mut Ui) {
        modal::caption(ui, "Base stats");
        ui.label(widgets::muted("StatResource").small());
        ui.add_space(4.0);
        egui::Grid::new("base-stats").num_columns(2).spacing([14.0, 7.0]).show(ui, |ui| {
            for stat in Stat::ALL {
                let dirty = self.draft.is_base_stat_dirty(stat);
                stat_label(ui, stat, dirty);
                ui.add(egui::DragValue::new(&mut self.draft.base[stat.index()]).speed(0.2).range(0..=1_000_000));
                ui.end_row();
            }
        });
    }

    fn growth_bonuses(&mut self, ui: &mut Ui) {
        modal::caption(ui, "Growth bonus per level");
        ui.label(widgets::muted("CreatureLevelBonus").small());
        ui.add_space(4.0);
        egui::Grid::new("growth-bonus").num_columns(1 + self.draft.bonuses.len()).spacing([12.0, 7.0]).show(ui, |ui| {
            ui.label("");
            for bonus in &self.draft.bonuses {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let stage = RichText::new(format!("Stage {}", bonus.stage_index)).font(theme::bold(13.0));
                    ui.add(egui::Label::new(stage).extend());
                    let id = widgets::muted(format!("ID {}", bonus.lookup_id)).small();
                    if bonus.exists_in_db {
                        ui.add(egui::Label::new(id).extend());
                    } else {
                        ui.add(egui::Label::new(id.color(color::WARNING)).extend())
                            .on_hover_text("No row in the database yet; it is created when a value is saved.");
                    }
                });
            }
            ui.end_row();

            for stat in Stat::ALL {
                let dirty = self.draft.bonuses.iter().any(|bonus| bonus.is_stat_dirty(stat));
                stat_label(ui, stat, dirty);
                for bonus in &mut self.draft.bonuses {
                    let value = &mut bonus.values[stat.index()];
                    let mut number = value.to_f64().unwrap_or(0.0);
                    let drag =
                        egui::DragValue::new(&mut number).speed(0.005).fixed_decimals(4).range(-10_000.0..=10_000.0);
                    if ui.add_sized([88.0, 22.0], drag).changed()
                        && let Some(updated) = Decimal::from_f64(number)
                    {
                        *value = round_bonus(updated);
                    }
                }
                ui.end_row();
            }
        });
    }

    fn calculator(&mut self, ui: &mut Ui, result: &ScalingResult) {
        modal::caption(ui, "Calculator");
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = 130.0;
            ui.label("Level");
            ui.add(egui::Slider::new(&mut self.level, MIN_LEVEL..=MAX_LEVEL));
            ui.add_space(10.0);
            ui.label("Enhance");
            ui.add(egui::Slider::new(&mut self.enhance_stage, 0..=MAX_ENHANCE_STAGE));
        });
        let percent = (result.stat_amplify * Decimal::from(100)).normalize();
        ui.label(widgets::muted(format!(
            "Level {} · enhance stage {} · stat amplify +{percent}%",
            result.effective_level, result.effective_enhance_stage
        )));
        ui.add_space(4.0);

        egui::Grid::new("scaling-result").num_columns(9).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
            for title in ["Stat", "Base", "Growth 1", "Growth 2", "Growth 3", "Bonus", "Raw", "Final", "Δ / lv"] {
                ui.label(RichText::new(title).small().color(color::TEXT_MUTED));
            }
            ui.end_row();
            for row in &result.rows {
                ui.label(RichText::new(row.stat.label()).font(theme::bold(13.0)).color(STAT_COLORS[row.stat.index()]));
                ui.label(row.base.to_string());
                for stage in row.stage {
                    ui.label(dimmed_zero(stage));
                }
                ui.label(dimmed_zero(row.bonus));
                ui.label(row.final_raw.to_string());
                ui.label(RichText::new(row.final_value.to_string()).font(theme::bold(13.5)));
                ui.label(RichText::new(format!("+{}", row.delta)).color(color::SUCCESS));
                ui.end_row();
            }
        });

        ui.add_space(6.0);
        self.chart(ui);
    }

    /// Final stat values across all levels, with a marker at the selected level.
    fn chart(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 170.0), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, color::INPUT_BG);

        let plot = rect.shrink2(Vec2::new(10.0, 10.0));
        let max_value = self.curve.iter().flatten().copied().max().unwrap_or(1).max(1) as f32;
        let x_of =
            |level: i32| plot.left() + plot.width() * (level - MIN_LEVEL) as f32 / (MAX_LEVEL - MIN_LEVEL) as f32;
        let y_of = |value: i32| plot.bottom() - plot.height() * value as f32 / max_value;

        // Evolution stage boundaries.
        for boundary in [50, 100] {
            painter.vline(x_of(boundary), plot.y_range(), Stroke::new(1.0, color::BORDER));
        }
        for stat in Stat::ALL {
            let points: Vec<egui::Pos2> = self
                .curve
                .iter()
                .enumerate()
                .map(|(index, values)| egui::pos2(x_of(MIN_LEVEL + index as i32), y_of(values[stat.index()])))
                .collect();
            painter.line(points, Stroke::new(1.5, STAT_COLORS[stat.index()]));
        }
        painter.vline(x_of(self.level), plot.y_range(), Stroke::new(1.0, Color32::WHITE));
        painter.text(
            plot.left_top(),
            egui::Align2::LEFT_TOP,
            format!("Final stats by level  ·  max {max_value:.0}"),
            egui::FontId::proportional(11.5),
            color::TEXT_MUTED,
        );

        // Clicking or dragging on the chart picks the level.
        if let Some(pointer) = response.interact_pointer_pos() {
            let fraction = ((pointer.x - plot.left()) / plot.width()).clamp(0.0, 1.0);
            self.level = MIN_LEVEL + (fraction * (MAX_LEVEL - MIN_LEVEL) as f32).round() as i32;
        }
        response.on_hover_text("Click or drag to choose the level");
    }
}

fn stat_label(ui: &mut Ui, stat: Stat, dirty: bool) {
    let text = RichText::new(stat.label()).font(theme::bold(13.0));
    ui.label(if dirty { text.color(color::DIRTY_MARK) } else { text.color(STAT_COLORS[stat.index()]) });
}

fn dimmed_zero(value: i32) -> RichText {
    let text = RichText::new(value.to_string());
    if value == 0 { text.color(color::TEXT_FAINT) } else { text }
}
