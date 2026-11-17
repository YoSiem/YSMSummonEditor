//! Scripted UI driver for development builds.
//!
//! Set `YSM_DEV_SCRIPT` to a `;`-separated list of steps and the application
//! plays them back, which makes it possible to capture screenshots of any
//! screen without clicking through it by hand:
//!
//! ```text
//! YSM_DEV_SCRIPT="select 1013; shot main.png; stats; wait 20; shot stats.png; quit"
//! ```
//!
//! Steps run only while nothing is loading. This module is compiled out of
//! release builds.

use std::collections::VecDeque;
use std::path::PathBuf;

use eframe::egui;
use ysm_core::model::EntryField;
use ysm_core::save_plan::SaveAnswer;

use super::EditorApp;
use crate::action::{Action, PickerPurpose};

/// Frames to let the UI settle between steps.
const SETTLE_FRAMES: u32 = 6;

pub(super) struct DevScript {
    steps: VecDeque<String>,
    wait: u32,
    /// A screenshot was requested and its pixels are awaited.
    pending_shot: Option<PathBuf>,
}

impl DevScript {
    pub(super) fn from_env() -> Option<Self> {
        let script = std::env::var("YSM_DEV_SCRIPT").ok()?;
        let steps = script.split(';').map(str::trim).filter(|step| !step.is_empty()).map(str::to_owned).collect();
        Some(Self { steps, wait: SETTLE_FRAMES * 3, pending_shot: None })
    }
}

impl EditorApp {
    pub(super) fn run_dev_script(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let Some(mut script) = self.dev_script.take() else {
            return;
        };
        ctx.request_repaint();

        if let Some(path) = &script.pending_shot {
            let image = ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(image) = image {
                let [width, height] = image.size;
                let saved = image::RgbaImage::from_raw(width as u32, height as u32, image.as_raw().to_vec())
                    .ok_or_else(|| "unexpected screenshot size".to_owned())
                    .and_then(|pixels| pixels.save(path).map_err(|error| error.to_string()));
                match saved {
                    Ok(()) => println!("dev-script: saved {}", path.display()),
                    Err(error) => println!("dev-script: could not save {}: {error}", path.display()),
                }
                script.pending_shot = None;
            }
            self.dev_script = Some(script);
            return;
        }

        let loading = self.state.summons_loading
            || self.state.chain_loading
            || self.busy.is_some()
            || !self.state.connection.is_ready()
            || self.state.asset_count.is_none();
        if loading {
            script.wait = SETTLE_FRAMES;
        } else if script.wait > 0 {
            script.wait -= 1;
        } else if let Some(step) = script.steps.pop_front() {
            println!("dev-script: {step}");
            script.wait = SETTLE_FRAMES;
            self.run_dev_step(ctx, &step, &mut script, actions);
        }
        self.dev_script = Some(script);
    }

    fn run_dev_step(&mut self, ctx: &egui::Context, step: &str, script: &mut DevScript, actions: &mut Vec<Action>) {
        let mut words = step.split_whitespace();
        let command = words.next().unwrap_or_default();
        let argument = words.next().unwrap_or_default();
        let number: i32 = argument.parse().unwrap_or(0);
        let tree = self.state.active_tree_ref();
        let selected = self.state.selected_row;

        match command {
            "wait" => script.wait = number.max(0) as u32,
            "shot" => {
                script.pending_shot = Some(PathBuf::from(argument));
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            "quit" => actions.push(Action::Exit),
            "search" => {
                self.state.search_text = step.strip_prefix("search").unwrap_or_default().trim().to_owned();
                actions.push(Action::Search);
            }
            "select" => {
                if let Some(summon) = self.state.summons.iter().find(|s| s.summon_id == number) {
                    actions.push(Action::SwitchSummon(summon.clone()));
                }
            }
            "click" => {
                if let Some(summon) = self.state.summons.iter().find(|s| s.summon_id == number) {
                    actions.push(Action::SelectSummon(summon.clone()));
                }
            }
            "stage" => actions.push(Action::SetStage(number as u8)),
            "slot" => actions.push(Action::SetSlot(number as u8)),
            "row" => {
                let uid = self.state.active_tree().and_then(|t| t.rows.get(number.max(0) as usize)).map(|row| row.uid);
                actions.push(Action::SelectRow(uid));
            }
            "set" => {
                let value = words.next().and_then(|text| text.parse().ok()).unwrap_or(0);
                let field = EntryField::ALL.into_iter().find(|field| field.column() == argument);
                if let (Some(uid), Some(field)) = (selected, field) {
                    actions.push(Action::SetField { tree, uid, field, value });
                }
            }
            "addrow" => actions.push(Action::AddRow(tree)),
            "duplicate" => actions.extend(selected.map(|uid| Action::DuplicateRow(tree, uid))),
            "delete" => actions.extend(selected.map(|uid| Action::DeleteRow(tree, uid))),
            "askdelete" => actions.extend(selected.map(|uid| Action::AskDeleteRow(tree, uid))),
            "undo" => actions.push(Action::Undo),
            "redo" => actions.push(Action::Redo),
            "picker" => actions.push(Action::OpenSkillPicker(PickerPurpose::AddRow(tree))),
            "editskill" => actions.push(Action::EditSkill(number)),
            "creategroup" => actions.extend(selected.map(|uid| Action::CreateGroup(tree, uid))),
            "editgroup" => actions.extend(selected.map(|uid| Action::EditGroup(tree, uid))),
            "stats" => actions.push(Action::OpenStats),
            "settings" => actions.push(Action::OpenSettings),
            "sqlmode" => actions.push(Action::SetSaveToSql(number != 0)),
            "save" => actions.push(Action::Save),
            "answer" => actions.push(Action::SaveAnswer(match argument {
                "clone" => SaveAnswer::Clone,
                "inplace" => SaveAnswer::InPlace,
                _ => SaveAnswer::Continue,
            })),
            "review" => actions.push(Action::ReviewConfirmed),
            "commit" => actions.push(Action::CommitConfirmed),
            // Closes the topmost dialog without triggering any of its buttons.
            "close" => {
                self.dialogs.pop();
            }
            other => println!("dev-script: unknown step '{other}'"),
        }
    }
}
