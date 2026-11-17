//! Modal dialogs.
//!
//! The application keeps a stack of [`Dialog`]s and draws them every frame;
//! the topmost one is interactive. Dialogs report results by pushing
//! [`Action`]s and learn about worker results through [`Dialog::on_event`].

pub mod message;
pub mod random_group;
pub mod save_review;
pub mod settings;
pub mod skill_edit;
pub mod skill_picker;
mod skill_search;
pub mod stat_scaling;

use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui;
use ysm_core::session::EditSession;

use crate::action::Action;
use crate::textures::TextureStore;
use crate::worker::Event;

pub use message::MessageDialog;

/// What a dialog may read and use while drawing.
pub struct DialogContext<'a> {
    pub session: &'a EditSession,
    pub textures: &'a mut TextureStore,
    pub actions: &'a mut Vec<Action>,
}

pub enum Dialog {
    Message(Box<MessageDialog>),
    Settings(Box<settings::SettingsDialog>),
    SkillEdit(Box<skill_edit::SkillEditDialog>),
    SkillPicker(Box<skill_picker::SkillPickerDialog>),
    RandomGroup(Box<random_group::RandomGroupDialog>),
    StatScaling(Box<stat_scaling::StatScalingDialog>),
    SaveReview(Box<save_review::SaveReviewDialog>),
}

impl Dialog {
    pub fn message(dialog: MessageDialog) -> Self {
        Dialog::Message(Box::new(dialog))
    }

    /// Draws the dialog. Returns `false` when it should be closed.
    pub fn show(&mut self, ctx: &egui::Context, id: egui::Id, env: &mut DialogContext<'_>) -> bool {
        match self {
            Dialog::Message(dialog) => dialog.show(ctx, id, env),
            Dialog::Settings(dialog) => dialog.show(ctx, id, env),
            Dialog::SkillEdit(dialog) => dialog.show(ctx, id, env),
            Dialog::SkillPicker(dialog) => dialog.show(ctx, id, env),
            Dialog::RandomGroup(dialog) => dialog.show(ctx, id, env),
            Dialog::StatScaling(dialog) => dialog.show(ctx, id, env),
            Dialog::SaveReview(dialog) => dialog.show(ctx, id, env),
        }
    }

    /// Lets the dialog pick up results of requests it sent to the worker.
    pub fn on_event(&mut self, event: &Event) {
        match self {
            Dialog::Settings(dialog) => dialog.on_event(event),
            Dialog::SkillPicker(dialog) => dialog.on_event(event),
            Dialog::RandomGroup(dialog) => dialog.on_event(event),
            Dialog::Message(_) | Dialog::SkillEdit(_) | Dialog::StatScaling(_) | Dialog::SaveReview(_) => {}
        }
    }
}

/// Process-wide generation counter, so each search can recognize its own answer.
pub fn next_generation() -> u64 {
    static GENERATION: AtomicU64 = AtomicU64::new(1);
    GENERATION.fetch_add(1, Ordering::Relaxed)
}
