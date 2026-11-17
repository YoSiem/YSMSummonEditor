//! Editor for the candidate skills of a random group.

use std::collections::HashMap;

use eframe::egui::{self, RichText, Ui};
use ysm_core::model::SkillDisplay;
use ysm_core::random_group::{MAX_GROUP_SKILLS, RandomSkillGroup};
use ysm_core::session::{EditSession, TreeRef};

use super::DialogContext;
use super::skill_search::{SearchInteraction, SkillSearch, skill_row};
use crate::action::Action;
use crate::icons;
use crate::theme::{self, color};
use crate::widgets::{self, modal};
use crate::worker::Event;

const LIST_HEIGHT: f32 = 400.0;

pub struct RandomGroupDialog {
    tree: TreeRef,
    uid: u64,
    group_id: i32,
    is_new: bool,
    original: Vec<i32>,
    /// Candidate skill IDs, kept sorted and distinct.
    members: Vec<i32>,
    selected_member: Option<i32>,
    /// Display data of skills seen in search results, so members keep their names.
    known: HashMap<i32, SkillDisplay>,
    search: SkillSearch,
    status: String,
}

impl RandomGroupDialog {
    /// `group` is the version to start from; `is_new` marks a group that does not exist yet.
    pub fn new(tree: TreeRef, uid: u64, group: &RandomSkillGroup, is_new: bool) -> Self {
        Self {
            tree,
            uid,
            group_id: group.group_id,
            is_new,
            original: group.skill_ids().to_vec(),
            members: group.skill_ids().to_vec(),
            selected_member: None,
            known: HashMap::new(),
            search: SkillSearch::new(),
            status: String::new(),
        }
    }

    pub fn on_event(&mut self, event: &Event) {
        if let Event::SkillsFound { result: Ok(found), .. } = event {
            self.known.extend(found.iter().map(|skill| (skill.skill_id, skill.clone())));
        }
        self.search.on_event(event);
    }

    fn display(&self, session: &EditSession, skill_id: i32) -> SkillDisplay {
        self.known.get(&skill_id).or_else(|| session.skill_display(skill_id)).cloned().unwrap_or_else(|| SkillDisplay {
            skill_id,
            skill_name: format!("Skill {skill_id}"),
            icon_file_name: None,
            tooltip_markup: None,
        })
    }

    fn add(&mut self, skill: &SkillDisplay) {
        if self.members.contains(&skill.skill_id) {
            self.status = format!("Skill {} is already in the group.", skill.skill_id);
        } else if self.members.len() >= MAX_GROUP_SKILLS {
            self.status = format!("A random group can hold at most {MAX_GROUP_SKILLS} skills.");
        } else {
            self.known.insert(skill.skill_id, skill.clone());
            self.members.push(skill.skill_id);
            self.members.sort_unstable();
            self.status = format!("Added skill {}.", skill.skill_id);
        }
    }

    fn remove(&mut self, skill_id: i32) {
        self.members.retain(|id| *id != skill_id);
        if self.selected_member == Some(skill_id) {
            self.selected_member = None;
        }
        self.status = format!("Removed skill {skill_id}.");
    }

    pub fn show(&mut self, ctx: &egui::Context, id: egui::Id, env: &mut DialogContext<'_>) -> bool {
        let title = if self.is_new {
            format!("New Random Group · RND[{}]", self.group_id)
        } else {
            format!("Random Group · RND[{}]", self.group_id)
        };
        let mut apply = false;
        let mut cancelled = false;

        let outcome = modal::show(ctx, id, icons::RANDOM_GROUP, color::ACCENT_BRIGHT, &title, 900.0, |ui| {
            ui.label(widgets::muted(
                "A tree row with this group grants one random skill out of the candidates. \
                 Candidates are kept sorted by skill ID and UseCode always equals their number.",
            ));
            ui.add_space(4.0);

            let column_width = (ui.available_width() - 130.0) / 2.0;
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(column_width);
                    modal::caption(ui, "All skills");
                    let members = self.members.clone();
                    let interaction = self.search.show(ui, env, LIST_HEIGHT, |skill_id| members.contains(&skill_id));
                    if let SearchInteraction::Activated(skill) = interaction {
                        self.add(&skill);
                    }
                });

                ui.vertical(|ui| {
                    ui.set_width(110.0);
                    ui.add_space(110.0);
                    let candidate = self.search.selected_skill().cloned();
                    let can_add = candidate.as_ref().is_some_and(|skill| !self.members.contains(&skill.skill_id))
                        && self.members.len() < MAX_GROUP_SKILLS;
                    if ui
                        .add_enabled(
                            can_add,
                            egui::Button::new(format!("Add  {}", icons::ARROW_RIGHT)).min_size([104.0, 30.0].into()),
                        )
                        .clicked()
                        && let Some(skill) = candidate
                    {
                        self.add(&skill);
                    }
                    if ui
                        .add_enabled(
                            self.selected_member.is_some(),
                            egui::Button::new(widgets::icon_text(icons::DELETE, "Remove"))
                                .min_size([104.0, 30.0].into()),
                        )
                        .clicked()
                        && let Some(skill_id) = self.selected_member
                    {
                        self.remove(skill_id);
                    }
                    if ui
                        .add_enabled(
                            !self.members.is_empty(),
                            egui::Button::new(widgets::icon_text(icons::CLOSE, "Clear")).min_size([104.0, 30.0].into()),
                        )
                        .clicked()
                    {
                        self.members.clear();
                        self.selected_member = None;
                        self.status = "Selection cleared.".to_owned();
                    }
                });

                ui.vertical(|ui| {
                    ui.set_width(column_width);
                    self.members_column(ui, env);
                });
            });

            modal::footer(ui, |ui| {
                if widgets::button(ui, icons::CLOSE, "Cancel").clicked() {
                    cancelled = true;
                }
                let changed = self.is_new || self.members != self.original;
                ui.add_enabled_ui(changed, |ui| {
                    if widgets::primary_button(ui, icons::CHECK, "Apply").clicked() {
                        apply = true;
                    }
                });
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(widgets::muted(self.status.as_str()));
                });
            });
        });

        if apply {
            env.actions.push(Action::ApplyRandomGroup {
                tree: self.tree,
                uid: self.uid,
                group: RandomSkillGroup::new(self.group_id, self.members.iter().copied()),
            });
            return false;
        }
        // A click outside must not throw away an edited group; only Escape and Cancel close it.
        let escaped = outcome.dismissed && ctx.input(|input| input.key_pressed(egui::Key::Escape));
        !(cancelled || escaped)
    }

    fn members_column(&mut self, ui: &mut Ui, env: &mut DialogContext<'_>) {
        let full = self.members.len() >= MAX_GROUP_SKILLS;
        ui.horizontal(|ui| {
            modal::caption(ui, "Group candidates");
            let count = format!("{} / {MAX_GROUP_SKILLS}", self.members.len());
            ui.label(RichText::new(count).font(theme::bold(13.5)).color(if full {
                color::WARNING
            } else {
                color::TEXT
            }));
        });
        // Keeps the two lists aligned with the search box and counter on the left.
        ui.add_space(58.0);

        let mut clicked = None;
        let mut removed = None;
        egui::Frame::new().fill(color::INPUT_BG).corner_radius(6).inner_margin(4).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            egui::ScrollArea::vertical()
                .id_salt("group-members")
                .auto_shrink(false)
                .max_height(LIST_HEIGHT)
                .min_scrolled_height(LIST_HEIGHT)
                .show(ui, |ui| {
                    if self.members.is_empty() {
                        widgets::empty_state(
                            ui,
                            icons::RANDOM_GROUP,
                            "No candidates yet",
                            "Double-click a skill on the left to add it.",
                        );
                    }
                    for (index, skill_id) in self.members.iter().enumerate() {
                        let display = self.display(env.session, *skill_id);
                        let selected = self.selected_member == Some(*skill_id);
                        let response = skill_row(ui, env, &display, selected, false, Some(index + 1));
                        if response.clicked() {
                            clicked = Some(*skill_id);
                        }
                        if response.double_clicked() {
                            removed = Some(*skill_id);
                        }
                    }
                });
        });

        if let Some(skill_id) = clicked {
            self.selected_member = Some(skill_id);
        }
        if let Some(skill_id) = removed {
            self.remove(skill_id);
        }
    }
}
