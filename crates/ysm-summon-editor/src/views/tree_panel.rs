//! Centre panel: evolution and tree tabs with the editable skill tree grid.

use eframe::egui::{self, Align, Layout, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};
use ysm_core::model::{EntryField, STAGE_COUNT, TREE_SLOTS};
use ysm_core::session::{EntryRow, TreeRef, TreeState};

use super::{ViewContext, header_cell, tab, value_color};
use crate::action::{Action, PickerPurpose};
use crate::icons;
use crate::theme::{self, color};
use crate::widgets::int_cell::int_cell;
use crate::widgets::{self, game_tooltip, icon_text, paint_row_background};

/// Width of a number column: IDs need room for five digits, levels do not.
fn column_width(field: EntryField) -> f32 {
    match field {
        EntryField::SkillId => 54.0,
        EntryField::SkillGroupId | EntryField::NeedSkillId(_) => 46.0,
        _ => 34.0,
    }
}

/// Room the horizontal scroll bar needs below the grid.
const SCROLL_BAR_ALLOWANCE: f32 = 14.0;

pub fn show(ui: &mut Ui, view: &mut ViewContext<'_>) {
    if !view.state.session.is_loaded() {
        theme::card().show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            widgets::empty_state(ui, icons::TREE, "No skill trees loaded", "Select a summon to edit its skill trees.");
        });
        return;
    }

    stage_tabs(ui, view);
    ui.add_space(2.0);
    theme::card().show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        tree_tabs(ui, view);

        let Some(tree) = view.state.active_tree() else {
            return;
        };
        action_bar(ui, view, tree);
        warnings(ui, view, tree);
        ui.add_space(2.0);

        if tree.tree_id <= 0 {
            widgets::empty_state(
                ui,
                icons::TREE,
                "No skill tree in this slot",
                "The summon has no tree assigned here.",
            );
        } else if tree.rows.is_empty() {
            widgets::empty_state(ui, icons::ADD_SKILL, "This tree is empty", "Use Add Row or Add Skill to start.");
        } else {
            grid(ui, view, tree);
        }
    });
}

fn stage_tabs(ui: &mut Ui, view: &mut ViewContext<'_>) {
    let state = view.state;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for stage in 1..=STAGE_COUNT as u8 {
            let summon = state.session.chain().and_then(|chain| chain.stage(stage));
            let dirty = state
                .session
                .stages()
                .iter()
                .filter(|s| s.stage == stage)
                .flat_map(|s| &s.trees)
                .any(TreeState::is_changed);
            let subtitle = summon.map_or_else(|| "—".to_owned(), |s| format!("{} · {}", s.summon_name, s.summon_id));
            let response =
                tab(ui, state.active_stage == stage, summon.is_some(), &format!("Evolution {stage}"), &subtitle, dirty);
            let response = match summon {
                Some(_) => response.on_hover_text(format!("Skill trees of evolution {stage}")),
                None => response.on_hover_text(format!("This chain has no evolution {stage}")),
            };
            if response.clicked() {
                view.actions.push(Action::SetStage(stage));
            }
        }
    });
}

fn tree_tabs(ui: &mut Ui, view: &mut ViewContext<'_>) {
    let state = view.state;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for slot in 1..=TREE_SLOTS as u8 {
            let tree = state.session.tree(TreeRef::new(state.active_stage, slot));
            let tree_id = tree.map_or(0, |t| t.tree_id);
            let subtitle = if tree_id > 0 { format!("#{tree_id}") } else { "—".to_owned() };
            let dirty = tree.is_some_and(TreeState::is_changed);
            let response =
                tab(ui, state.active_slot == slot, tree_id > 0, &format!("SkillTree {slot}"), &subtitle, dirty)
                    .on_hover_text(format!("Tree slot {slot}, skill_tree_id: {tree_id}"));
            if response.clicked() {
                view.actions.push(Action::SetSlot(slot));
            }
        }
    });
    ui.separator();
}

fn action_bar(ui: &mut Ui, view: &mut ViewContext<'_>, tree: &TreeState) {
    let selected = view.state.selected_row.and_then(|uid| tree.row(uid));
    let editable = tree.is_editable();

    ui.horizontal(|ui| {
        if ui
            .add_enabled(editable, egui::Button::new(icon_text(icons::ADD, "Add Row")))
            .on_hover_text("Append an empty row (Insert)")
            .clicked()
        {
            view.actions.push(Action::AddRow(tree.tree));
        }
        if ui
            .add_enabled(editable, egui::Button::new(icon_text(icons::ADD_SKILL, "Add Skill…")))
            .on_hover_text("Search for a skill by name and append it")
            .clicked()
        {
            view.actions.push(Action::OpenSkillPicker(PickerPurpose::AddRow(tree.tree)));
        }

        ui.separator();
        let has_row = selected.is_some();
        if ui.add_enabled(has_row, egui::Button::new(icon_text(icons::DUPLICATE, "Duplicate"))).clicked()
            && let Some(row) = selected
        {
            view.actions.push(Action::DuplicateRow(tree.tree, row.uid));
        }
        if ui
            .add_enabled(has_row, egui::Button::new(icon_text(icons::DELETE, "Delete")))
            .on_hover_text("Delete the selected row (Del)")
            .clicked()
            && let Some(row) = selected
        {
            view.actions.push(Action::AskDeleteRow(tree.tree, row.uid));
        }
        let has_skill = selected.is_some_and(|row| row.entry.skill_id > 0);
        if ui.add_enabled(has_skill, egui::Button::new(icon_text(icons::EDIT, "Edit Skill"))).clicked()
            && let Some(row) = selected
        {
            view.actions.push(Action::EditSkill(row.entry.skill_id));
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let dirty = tree.rows.iter().filter(|row| row.is_dirty()).count();
            let mut summary = format!("{} rows", tree.rows.len());
            if dirty > 0 {
                summary.push_str(&format!(" · {dirty} changed"));
            }
            ui.label(widgets::muted(summary).small());
        });
    });
}

fn warnings(ui: &mut Ui, view: &ViewContext<'_>, tree: &TreeState) {
    let shared = view.state.session.trees_sharing_id(tree.tree);
    if shared.is_empty() {
        return;
    }
    let places: Vec<String> = shared.iter().map(|t| format!("Evolution {} / SkillTree {}", t.stage, t.slot)).collect();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(icons::WARNING).color(color::WARNING));
        ui.label(
            RichText::new(format!(
                "Tree #{} is also used by {} in this chain. Edit it in one place only — each tab is saved separately.",
                tree.tree_id,
                places.join(", ")
            ))
            .color(color::WARNING)
            .small(),
        );
    });
}

fn grid(ui: &mut Ui, view: &mut ViewContext<'_>, tree: &TreeState) {
    let text_height = ui.text_style_height(&egui::TextStyle::Body);
    let available_height = ui.available_height() - SCROLL_BAR_ALLOWANCE;

    egui::ScrollArea::horizontal().id_salt(("tree-scroll", tree.tree)).auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let mut table = TableBuilder::new(ui)
            .id_salt(("tree-grid", tree.tree))
            .sense(Sense::click())
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .auto_shrink(false)
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .column(Column::exact(theme::ICON_SIZE + 4.0))
            .column(Column::exact(column_width(EntryField::SkillId)))
            .column(Column::remainder().at_least(150.0).clip(true));
        for field in EntryField::ALL.into_iter().skip(1) {
            table = table.column(Column::exact(column_width(field)));
        }

        table
            .header(text_height + 10.0, |mut header| {
                header.col(|_| {});
                header.col(|ui| header_cell(ui, EntryField::SkillId.header(), EntryField::SkillId.description()));
                header.col(|ui| header_cell(ui, "Skill", "Skill name; double-click to edit the skill"));
                for field in EntryField::ALL.into_iter().skip(1) {
                    header.col(|ui| header_cell(ui, field.header(), field.description()));
                }
            })
            .body(|body| {
                body.rows(theme::ROW_HEIGHT, tree.rows.len(), |mut table_row| {
                    let row = &tree.rows[table_row.index()];
                    grid_row(&mut table_row, view, tree, row);
                });
            });
    });
}

fn grid_row(
    table_row: &mut egui_extras::TableRow<'_, '_>,
    view: &mut ViewContext<'_>,
    tree: &TreeState,
    row: &EntryRow,
) {
    let state = view.state;
    let session = &state.session;
    let selected = state.selected_row == Some(row.uid);
    let dirty_fill = (row.is_dirty() && !selected).then_some(color::ROW_DIRTY);
    let skill_id = row.entry.skill_id;
    let display = session.skill_display(skill_id);
    table_row.set_selected(selected);

    let menu = |ui: &mut Ui, actions: &mut Vec<Action>| row_menu(ui, actions, tree, row);
    let select = |response: &egui::Response, actions: &mut Vec<Action>| {
        if (response.clicked() || response.secondary_clicked() || response.gained_focus()) && !selected {
            actions.push(Action::SelectRow(Some(row.uid)));
        }
    };

    // Icon
    table_row.col(|ui| {
        if let Some(fill) = dirty_fill {
            paint_row_background(ui, fill);
        }
        let icon = view.textures.icon(display.and_then(|d| d.icon_file_name.as_deref()));
        let response = widgets::game_icon(ui, icon, theme::ICON_SIZE);
        if skill_id > 0 {
            response.on_hover_ui(|ui| game_tooltip::skill(ui, skill_id, display));
        }
    });

    // Skill ID
    table_row.col(|ui| {
        if let Some(fill) = dirty_fill {
            paint_row_background(ui, fill);
        }
        number_cell(ui, view.actions, tree, row, EntryField::SkillId, selected, &menu, &select);
    });

    // Skill name and random group badge
    let (_, name_response) = table_row.col(|ui| {
        if let Some(fill) = dirty_fill {
            paint_row_background(ui, fill);
        }
        let (name, tint) = if skill_id <= 0 {
            ("New skill".to_owned(), color::TEXT_FAINT)
        } else if display.is_none() {
            (session.skill_name(skill_id), color::WARNING)
        } else {
            (session.skill_name(skill_id), color::TEXT)
        };
        let label = ui.add(egui::Label::new(RichText::new(name).color(tint)).truncate().selectable(false));
        if skill_id > 0 {
            label.on_hover_ui(|ui| game_tooltip::skill(ui, skill_id, display));
        }

        let group_id = row.entry.skill_group_id;
        if group_id > 0 {
            widgets::badge(ui, &format!("RND[{group_id}]"), color::ACCENT_BRIGHT, color::PANEL_ALT)
                .on_hover_ui(|ui| game_tooltip::random_group(ui, session, group_id));
        }
    });
    if name_response.double_clicked() && skill_id > 0 {
        view.actions.push(Action::EditSkill(skill_id));
    }

    for field in EntryField::ALL.into_iter().skip(1) {
        table_row.col(|ui| {
            if let Some(fill) = dirty_fill {
                paint_row_background(ui, fill);
            }
            number_cell(ui, view.actions, tree, row, field, selected, &menu, &select);
        });
    }

    let response = table_row.response();
    select(&response, view.actions);
    response.context_menu(|ui| menu(ui, view.actions));
}

fn number_cell(
    ui: &mut Ui,
    actions: &mut Vec<Action>,
    tree: &TreeState,
    row: &EntryRow,
    field: EntryField,
    selected: bool,
    menu: &impl Fn(&mut Ui, &mut Vec<Action>),
    select: &impl Fn(&egui::Response, &mut Vec<Action>),
) {
    let value = row.entry.get(field);
    let changed = row.original.is_none_or(|original| original.get(field) != value);
    let id = egui::Id::new(("tree-cell", row.uid, field));

    // Dimmed zeros would vanish on the selection colour.
    let tint = if selected && !changed && value == 0 { color::TEXT_MUTED } else { value_color(value, changed) };
    let output = int_cell(ui, id, value, tint);
    select(&output.response, actions);
    if let Some(value) = output.committed {
        actions.push(Action::SetField { tree: tree.tree, uid: row.uid, field, value });
    }
    output.response.context_menu(|ui| menu(ui, actions));
}

fn row_menu(ui: &mut Ui, actions: &mut Vec<Action>, tree: &TreeState, row: &EntryRow) {
    ui.set_min_width(210.0);
    let has_skill = row.entry.skill_id > 0;
    let has_group = row.entry.skill_group_id > 0;
    let mut item = |ui: &mut Ui, enabled: bool, icon: &str, text: &str, action: Action| {
        if ui.add_enabled(enabled, egui::Button::new(icon_text(icon, text))).clicked() {
            actions.push(action);
            ui.close();
        }
    };

    item(ui, tree.is_editable(), icons::ADD, "Add Row", Action::AddRow(tree.tree));
    item(
        ui,
        tree.is_editable(),
        icons::ADD_SKILL,
        "Add Skill…",
        Action::OpenSkillPicker(PickerPurpose::AddRow(tree.tree)),
    );
    item(
        ui,
        true,
        icons::SWAP,
        "Change Skill…",
        Action::OpenSkillPicker(PickerPurpose::ChangeSkill(tree.tree, row.uid)),
    );
    item(ui, true, icons::DUPLICATE, "Duplicate Row", Action::DuplicateRow(tree.tree, row.uid));
    item(ui, has_skill, icons::EDIT, "Edit Skill", Action::EditSkill(row.entry.skill_id));
    item(ui, true, icons::DELETE, "Delete From Tree", Action::AskDeleteRow(tree.tree, row.uid));
    ui.separator();
    item(ui, !has_group, icons::RANDOM_GROUP, "Create Random Group", Action::CreateGroup(tree.tree, row.uid));
    item(ui, has_group, icons::RANDOM_GROUP, "Edit Random Group", Action::EditGroup(tree.tree, row.uid));
    item(ui, has_group, icons::DETACH, "Detach Random Group", Action::AskDetachGroup(tree.tree, row.uid));
}

#[cfg(test)]
mod tests {
    use eframe::egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use ysm_data::{AppSettings, InMemoryRepository, SummonRepository, loader};

    use super::*;
    use crate::state::AppState;
    use crate::textures::TextureStore;

    struct Fixture {
        state: AppState,
        textures: Option<TextureStore>,
        actions: Vec<Action>,
    }

    /// The tree panel showing the first tree of the demo summon 1013.
    fn harness<'a>() -> Harness<'a, Fixture> {
        let mut repo = InMemoryRepository::demo();
        let summon = repo.summon_by_id(1013).unwrap().unwrap();
        let mut state = AppState::new(AppSettings::default(), true);
        state.session.load(loader::load_chain(&mut repo, &summon).unwrap());
        state.active_stage = 3;

        Harness::builder().with_size([1100.0, 700.0]).build_ui_state(
            |ui, fixture: &mut Fixture| {
                // Fonts set through the context apply from the next frame, so the first one draws nothing.
                let Some(textures) = fixture.textures.as_mut() else {
                    crate::theme::apply(ui.ctx());
                    fixture.textures = Some(TextureStore::new(ui.ctx()));
                    return;
                };
                let mut view = ViewContext { state: &fixture.state, textures, actions: &mut fixture.actions };
                show(ui, &mut view);
            },
            Fixture { state, textures: None, actions: Vec::new() },
        )
    }

    fn first_row_uid(harness: &Harness<'_, Fixture>) -> u64 {
        harness.state().state.active_tree().unwrap().rows[0].uid
    }

    #[test]
    fn right_click_on_a_cell_opens_the_row_menu() {
        let mut harness = harness();
        harness.run();
        let uid = first_row_uid(&harness);

        harness.get_all_by_role(Role::TextInput).next().unwrap().click_secondary();
        harness.run();
        harness.get_by_label_contains("Delete From Tree").click();
        harness.run();

        let actions = &harness.state().actions;
        assert!(actions.iter().any(|a| matches!(a, Action::SelectRow(Some(selected)) if *selected == uid)));
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::AskDeleteRow(tree, row) if *tree == TreeRef::new(3, 1) && *row == uid))
        );
    }

    #[test]
    fn typing_in_a_cell_pushes_a_field_edit() {
        let mut harness = harness();
        harness.run();
        let uid = first_row_uid(&harness);

        // The first text input of a row is its skill ID.
        harness.get_all_by_role(Role::TextInput).next().unwrap().focus();
        harness.run();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        harness.run();
        harness.get_all_by_role(Role::TextInput).next().unwrap().type_text("40010");
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();

        assert!(harness.state().actions.iter().any(|a| matches!(
            a,
            Action::SetField { uid: row, field: EntryField::SkillId, value: 40010, .. } if *row == uid
        )));
    }

    #[test]
    fn add_row_button_is_wired() {
        let mut harness = harness();
        harness.run();
        harness.get_by_label_contains("Add Row").click();
        harness.run();

        assert!(
            harness.state().actions.iter().any(|a| matches!(a, Action::AddRow(tree) if *tree == TreeRef::new(3, 1)))
        );
    }
}
