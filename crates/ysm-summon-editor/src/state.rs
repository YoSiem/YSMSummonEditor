//! Everything the views read to draw a frame.

use std::time::Instant;

use ysm_core::model::{SummonListFilter, SummonListItem};
use ysm_core::session::{EditSession, TreeRef, TreeState};
use ysm_data::AppSettings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    NotConfigured,
    Connecting,
    /// Connected; carries a short server description.
    Connected(String),
    Failed(String),
    /// Running on built-in sample data.
    Demo,
}

impl ConnectionStatus {
    pub fn is_ready(&self) -> bool {
        matches!(self, ConnectionStatus::Connected(_) | ConnectionStatus::Demo)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ListFilter {
    pub include_riding_only: bool,
    pub final_only: bool,
}

pub struct AppState {
    pub settings: AppSettings,
    pub demo: bool,
    pub connection: ConnectionStatus,
    /// Number of indexed image files; `None` while the dump folder is being scanned.
    pub asset_count: Option<usize>,

    pub search_text: String,
    /// When the search box was last edited, for debounced live search.
    pub search_edited_at: Option<Instant>,
    /// Set to move keyboard focus into the search box on the next frame.
    pub focus_search: bool,
    pub filter: ListFilter,
    pub summons: Vec<SummonListItem>,
    pub summons_loading: bool,
    /// The summon whose chain is loaded (or loading).
    pub selected_summon: Option<i32>,
    pub chain_loading: bool,

    pub session: EditSession,
    /// 1-based evolution stage shown in the tree panel.
    pub active_stage: u8,
    /// 1-based tree slot shown in the tree panel.
    pub active_slot: u8,
    pub selected_row: Option<u64>,
    /// Selected effective skill as `(skill_id, lv)`.
    pub selected_effective: Option<(i32, i32)>,

    pub status: String,
}

impl AppState {
    pub fn new(settings: AppSettings, demo: bool) -> Self {
        Self {
            settings,
            demo,
            connection: ConnectionStatus::NotConfigured,
            asset_count: None,
            search_text: String::new(),
            search_edited_at: None,
            focus_search: false,
            filter: ListFilter::default(),
            summons: Vec::new(),
            summons_loading: false,
            selected_summon: None,
            chain_loading: false,
            session: EditSession::new(),
            active_stage: 1,
            active_slot: 1,
            selected_row: None,
            selected_effective: None,
            status: String::new(),
        }
    }

    pub fn active_tree_ref(&self) -> TreeRef {
        TreeRef::new(self.active_stage, self.active_slot)
    }

    pub fn active_tree(&self) -> Option<&TreeState> {
        self.session.tree(self.active_tree_ref())
    }

    pub fn summon_filter(&self) -> SummonListFilter {
        SummonListFilter {
            search_text: self.search_text.trim().to_owned(),
            include_riding_only: self.filter.include_riding_only,
            include_non_form1: true,
            show_all_evolutions: !self.filter.final_only,
        }
    }

    /// The stat editor works on the final evolution of the loaded chain.
    pub fn can_open_stats(&self) -> bool {
        self.connection.is_ready() && self.session.is_loaded()
    }
}
