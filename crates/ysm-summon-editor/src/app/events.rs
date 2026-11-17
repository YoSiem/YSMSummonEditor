//! Applies results coming back from the worker thread.

use ysm_core::random_group::{RandomSkillGroup, next_free_group_id};

use super::EditorApp;
use crate::dialogs::random_group::RandomGroupDialog;
use crate::dialogs::skill_edit::SkillEditDialog;
use crate::dialogs::stat_scaling::StatScalingDialog;
use crate::dialogs::{Dialog, MessageDialog};
use crate::state::ConnectionStatus;
use crate::widgets::toast::ToastKind;
use crate::worker::Event;

impl EditorApp {
    pub(super) fn handle_event(&mut self, event: Event) {
        match event {
            Event::Connected(Ok(server)) => {
                self.state.connection =
                    if self.state.demo { ConnectionStatus::Demo } else { ConnectionStatus::Connected(server) };
                self.search_summons();
            }
            Event::Connected(Err(error)) => {
                self.state.connection = ConnectionStatus::Failed(error.clone());
                self.state.summons.clear();
                self.set_status("Database connection failed (see details).");
                self.push_dialog(Dialog::message(MessageDialog::error(
                    "Database",
                    "Could not connect to the database. Check the connection in Settings.",
                    error,
                )));
            }
            // Only the settings dialog cares; it has already seen the event.
            Event::ConnectionTested(_) | Event::SkillsFound { .. } => {}

            Event::AssetIndexBuilt(index) => {
                self.state.asset_count = Some(index.len());
                self.textures.set_index(index);
            }

            Event::SummonsFound { generation, result } => {
                if generation != self.summon_generation {
                    return;
                }
                self.state.summons_loading = false;
                match result {
                    Ok(summons) => {
                        self.set_status(format!("Loaded {} summons.", summons.len()));
                        self.state.summons = summons;
                    }
                    Err(error) => {
                        self.set_status("Search failed (see details).");
                        self.push_dialog(Dialog::message(MessageDialog::error(
                            "Search failed",
                            "Search query failed.",
                            error,
                        )));
                    }
                }
            }

            Event::ChainLoaded { generation, summon, result } => {
                if generation != self.chain_generation {
                    return;
                }
                self.state.chain_loading = false;
                match result {
                    Ok(bundle) => {
                        // Open the tab of the summon that was clicked, not always the first stage.
                        let stage = (1..=3)
                            .find(|stage| bundle.chain.stage(*stage).is_some_and(|s| s.summon_id == summon.summon_id));
                        self.state.session.load(*bundle);
                        self.state.active_stage = stage.unwrap_or(3);
                        self.state.active_slot = 1;
                        self.set_status("Chain loaded.");
                    }
                    Err(error) => {
                        self.set_status("Chain load failed (see details).");
                        self.push_dialog(Dialog::message(MessageDialog::error(
                            "Chain load failed",
                            "Could not load the evolution chain.",
                            error,
                        )));
                    }
                }
            }

            Event::SkillDisplayLoaded(Ok(found)) => self.state.session.ingest_skill_display(found),
            Event::SkillDisplayLoaded(Err(error)) => {
                self.toasts.push(ToastKind::Warning, format!("Skill names could not be loaded. {error}"));
            }

            Event::RandomGroupsLoaded { requested, result: Ok(found) } => {
                self.state.session.ingest_random_groups(&requested, found);
            }
            Event::RandomGroupsLoaded { result: Err(error), .. } => {
                self.toasts.push(ToastKind::Warning, format!("Random groups could not be loaded. {error}"));
            }

            Event::SkillRecordLoaded { skill_id, result } => match result {
                Ok(record) => {
                    self.set_status(format!("Skill {skill_id} loaded."));
                    let session = &self.state.session;
                    let dialog =
                        SkillEditDialog::new(record, session.pending_skill(skill_id), session.skill_name(skill_id));
                    self.push_dialog(Dialog::SkillEdit(Box::new(dialog)));
                }
                Err(error) => self.push_dialog(Dialog::message(MessageDialog::error(
                    "Skill edit failed",
                    "Could not open the skill editor.",
                    error,
                ))),
            },

            Event::ScalingLoaded(Ok((db, creature_enhance))) => {
                let staged = self.state.session.pending_scaling(&db);
                let name = self
                    .state
                    .session
                    .chain()
                    .map_or_else(|| format!("Summon {}", db.summon_id), |chain| chain.stage3.summon_name.clone());
                let dialog = StatScalingDialog::new(name, &db, &staged, &creature_enhance);
                self.push_dialog(Dialog::StatScaling(Box::new(dialog)));
                self.set_status("Stat scaling loaded.");
            }
            Event::ScalingLoaded(Err(error)) => {
                self.set_status("Stat scaling open failed (see details).");
                self.push_dialog(Dialog::message(MessageDialog::error(
                    "Stat scaling",
                    "Could not open the stat scaling editor.",
                    error,
                )));
            }

            Event::NextRandomGroupId { tree, uid, result } => match result {
                Ok(db_next) => {
                    let group_id = next_free_group_id(db_next, &self.state.session.reserved_group_ids());
                    let dialog = RandomGroupDialog::new(tree, uid, &RandomSkillGroup::empty(group_id), true);
                    self.push_dialog(Dialog::RandomGroup(Box::new(dialog)));
                }
                Err(error) => self.push_dialog(Dialog::message(MessageDialog::error(
                    "Random group",
                    "Could not allocate a new random group ID.",
                    error,
                ))),
            },

            Event::SaveAnalyzed(result) => self.on_save_analyzed(result),
            Event::Committed(result) => self.on_committed(result),
        }
    }
}
