//! Debounced skill search shared by the skill picker and the random group editor.

use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, CornerRadius, FontId, Key, Margin, Sense, Stroke, StrokeKind, Ui, Vec2};
use ysm_core::model::SkillDisplay;

use super::{DialogContext, next_generation};
use crate::action::Action;
use crate::icons;
use crate::theme::{self, color};
use crate::widgets::{self, game_tooltip};
use crate::worker::{Command, Event};

const DEBOUNCE: Duration = Duration::from_millis(250);
const ROW_HEIGHT: f32 = 42.0;

/// What the user did with the result list this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchInteraction {
    None,
    /// Double-click, or Enter in the search box.
    Activated(SkillDisplay),
}

pub struct SkillSearch {
    query: String,
    edited_at: Option<Instant>,
    /// Generation of the request whose answer is awaited.
    awaiting: Option<u64>,
    started: bool,
    results: Vec<SkillDisplay>,
    error: Option<String>,
    pub selected: Option<i32>,
}

impl SkillSearch {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            edited_at: None,
            awaiting: None,
            started: false,
            results: Vec::new(),
            error: None,
            selected: None,
        }
    }

    pub fn selected_skill(&self) -> Option<&SkillDisplay> {
        self.results.iter().find(|skill| Some(skill.skill_id) == self.selected)
    }

    pub fn on_event(&mut self, event: &Event) {
        let Event::SkillsFound { generation, result } = event else {
            return;
        };
        if self.awaiting != Some(*generation) {
            return;
        }
        self.awaiting = None;
        match result {
            Ok(found) => {
                self.results = found.clone();
                self.error = None;
                if self.selected_skill().is_none() {
                    self.selected = None;
                }
            }
            Err(error) => {
                self.results.clear();
                self.error = Some(error.clone());
            }
        }
    }

    fn request(&mut self, actions: &mut Vec<Action>) {
        let generation = next_generation();
        self.awaiting = Some(generation);
        self.edited_at = None;
        actions.push(Action::Command(Command::SearchSkills { text: self.query.trim().to_owned(), generation }));
    }

    /// Draws the search box and the result list. `is_disabled` greys out skills that cannot be picked.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        env: &mut DialogContext<'_>,
        list_height: f32,
        is_disabled: impl Fn(i32) -> bool,
    ) -> SearchInteraction {
        let mut interaction = SearchInteraction::None;

        let search = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text(format!("{}  Skill ID or name", icons::SEARCH))
                .desired_width(f32::INFINITY)
                .margin(Margin::symmetric(8, 6)),
        );
        if !self.started {
            self.started = true;
            search.request_focus();
            self.request(env.actions);
        }
        if search.changed() {
            self.edited_at = Some(Instant::now());
        }
        let enter = search.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        if enter {
            // Enter confirms the highlighted (or only sensible) result.
            let candidate = self.selected_skill().or_else(|| self.results.first()).cloned();
            match candidate {
                Some(skill) if self.edited_at.is_none() && !is_disabled(skill.skill_id) => {
                    interaction = SearchInteraction::Activated(skill);
                }
                _ => self.request(env.actions),
            }
            search.request_focus();
        } else if let Some(edited_at) = self.edited_at {
            if edited_at.elapsed() >= DEBOUNCE {
                self.request(env.actions);
            } else {
                ui.ctx().request_repaint_after(DEBOUNCE - edited_at.elapsed());
            }
        }

        ui.horizontal(|ui| {
            if self.awaiting.is_some() {
                ui.spinner();
                ui.label(widgets::muted("Searching…").small());
            } else if let Some(error) = &self.error {
                ui.colored_label(color::DANGER, format!("{}  {error}", icons::ERROR));
            } else {
                ui.label(widgets::muted(format!("{} skills", self.results.len())).small());
            }
        });

        egui::Frame::new().fill(color::INPUT_BG).corner_radius(6).inner_margin(4).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            egui::ScrollArea::vertical()
                .id_salt("skill-search-results")
                .auto_shrink(false)
                .max_height(list_height)
                .min_scrolled_height(list_height)
                .show_rows(ui, ROW_HEIGHT, self.results.len(), |ui, range| {
                    for skill in &self.results[range] {
                        let disabled = is_disabled(skill.skill_id);
                        let selected = self.selected == Some(skill.skill_id);
                        let response = skill_row(ui, env, skill, selected, disabled, None);
                        if response.clicked() {
                            self.selected = Some(skill.skill_id);
                        }
                        if response.double_clicked() && !disabled {
                            interaction = SearchInteraction::Activated(skill.clone());
                        }
                    }
                });
        });

        interaction
    }
}

/// One skill in a list: icon, name, ID and an optional leading slot number.
pub fn skill_row(
    ui: &mut Ui,
    env: &mut DialogContext<'_>,
    skill: &SkillDisplay,
    selected: bool,
    disabled: bool,
    slot: Option<usize>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    if selected {
        ui.painter().rect_filled(rect, CornerRadius::same(theme::RADIUS), color::ROW_SELECTED);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, CornerRadius::same(theme::RADIUS), color::ROW_HOVER);
    }

    let mut left = rect.left() + 6.0;
    if let Some(slot) = slot {
        ui.painter().text(
            egui::pos2(left + 10.0, rect.center().y),
            Align2::CENTER_CENTER,
            slot.to_string(),
            FontId::proportional(12.0),
            color::TEXT_MUTED,
        );
        left += 26.0;
    }

    let icon_size = 32.0;
    let icon_rect =
        egui::Rect::from_min_size(egui::pos2(left, rect.center().y - icon_size / 2.0), Vec2::splat(icon_size));
    let radius = CornerRadius::same(4);
    let texture = env.textures.icon(skill.icon_file_name.as_deref());
    let tint = if disabled { egui::Color32::from_gray(110) } else { egui::Color32::WHITE };
    egui::Image::from_texture((texture, icon_rect.size())).corner_radius(radius).tint(tint).paint_at(ui, icon_rect);
    ui.painter().rect_stroke(icon_rect, radius, Stroke::new(1.0, color::CONTROL_BORDER), StrokeKind::Inside);

    let text_left = icon_rect.right() + 10.0;
    let painter = ui.painter().with_clip_rect(rect.shrink2(Vec2::new(6.0, 0.0)));
    let name_color = if disabled { color::TEXT_FAINT } else { color::TEXT };
    painter.text(
        egui::pos2(text_left, rect.center().y - 8.0),
        Align2::LEFT_CENTER,
        &skill.skill_name,
        FontId::proportional(13.5),
        name_color,
    );
    let detail =
        if disabled { format!("{}  ·  already in the group", skill.skill_id) } else { skill.skill_id.to_string() };
    painter.text(
        egui::pos2(text_left, rect.center().y + 9.0),
        Align2::LEFT_CENTER,
        detail,
        FontId::proportional(11.5),
        color::TEXT_MUTED,
    );

    response.on_hover_ui(|ui| game_tooltip::skill(ui, skill.skill_id, Some(skill)))
}
