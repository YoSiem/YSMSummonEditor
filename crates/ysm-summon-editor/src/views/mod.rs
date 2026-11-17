//! The panels of the main window. Each view reads [`AppState`] and pushes
//! [`Action`]s; none of them changes application state directly.
//!
//! Layout of the window, with an 8 px gap between every pair of cards:
//!
//! ```text
//! ┌ toolbar ───────────────────────────────────────────────┐
//! ├ summons ┬ summon header ───────────────────────────────┤
//! │         ├ evolution tabs (apply to both cards below) ──┤
//! │         ├ skill tree card ───────┬ effective skills ───┤
//! ├ status bar ────────────────────────────────────────────┘
//! ```
//!
//! The two lower cards share one structure (title row, divider, toolbar
//! row, grid) so their grids start on the same line.

pub mod effective_panel;
pub mod stage_tabs;
pub mod status_bar;
pub mod summon_header;
pub mod summon_list;
pub mod toolbar;
pub mod tree_panel;

use eframe::egui::Ui;

use crate::action::Action;
use crate::state::AppState;
use crate::textures::TextureStore;
use crate::theme::space;
use crate::widgets;

/// What every view needs to draw itself.
pub struct ViewContext<'a> {
    pub state: &'a AppState,
    pub textures: &'a mut TextureStore,
    pub actions: &'a mut Vec<Action>,
}

/// Starts a grid card. Both grid cards then draw the same three things in
/// order: a title row, [`card_divider`] and a toolbar row, each row of the
/// standard control height, which is what makes their grids line up.
pub(crate) fn begin_card(ui: &mut Ui) {
    ui.spacing_mut().item_spacing.y = space::SM;
}

/// The hairline between the title row and the toolbar row of a grid card.
pub(crate) fn card_divider(ui: &mut Ui) {
    widgets::divider(ui);
}
