//! Background thread that owns the repository.
//!
//! The UI sends [`Command`]s and receives [`Event`]s; nothing on the UI
//! thread ever waits for the database or the file system. Requests that can
//! be superseded (searches, chain loads) carry a generation number so stale
//! answers can be ignored by the receiver.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;
use ysm_core::commit::{CommitRequest, CommitResult};
use ysm_core::model::{CreatureEnhance, SkillDisplay, StatScalingSnapshot, SummonListFilter, SummonListItem};
use ysm_core::random_group::RandomSkillGroup;
use ysm_core::save_plan::UsageReport;
use ysm_core::session::{ChainBundle, TreeRef};
use ysm_core::value::SkillRecord;
use ysm_data::repository::SKILL_SEARCH_LIMIT;
use ysm_data::{AssetIndex, DbSettings, InMemoryRepository, MssqlRepository, SummonRepository, loader};

/// Errors cross the thread boundary as display text; the UI only shows them.
pub type Outcome<T> = Result<T, String>;

pub enum Command {
    /// Switch to the MSSQL database described by the settings and verify it.
    Connect(DbSettings),
    /// Switch to the built-in demo data.
    UseDemo,
    /// Try a connection without touching the active repository.
    TestConnection(DbSettings),
    BuildAssetIndex(PathBuf),
    SearchSummons {
        filter: SummonListFilter,
        generation: u64,
    },
    LoadChain {
        summon: SummonListItem,
        generation: u64,
    },
    LoadSkillDisplay {
        ids: Vec<i32>,
    },
    LoadRandomGroups {
        ids: Vec<i32>,
    },
    LoadSkillRecord {
        skill_id: i32,
    },
    LoadScaling {
        summon_id: i32,
    },
    SearchSkills {
        text: String,
        generation: u64,
    },
    /// Ask for the next free random group ID, to create a group for a tree row.
    NextRandomGroupId {
        tree: TreeRef,
        uid: u64,
    },
    AnalyzeSave {
        request: CommitRequest,
        session_tree_ids: Vec<i32>,
    },
    Commit {
        request: CommitRequest,
    },
}

pub enum Event {
    Connected(Outcome<String>),
    ConnectionTested(Outcome<String>),
    AssetIndexBuilt(Arc<AssetIndex>),
    SummonsFound { generation: u64, result: Outcome<Vec<SummonListItem>> },
    ChainLoaded { generation: u64, summon: SummonListItem, result: Outcome<Box<ChainBundle>> },
    SkillDisplayLoaded(Outcome<Vec<SkillDisplay>>),
    RandomGroupsLoaded { requested: Vec<i32>, result: Outcome<Vec<RandomSkillGroup>> },
    SkillRecordLoaded { skill_id: i32, result: Outcome<SkillRecord> },
    ScalingLoaded(Outcome<(StatScalingSnapshot, Vec<CreatureEnhance>)>),
    SkillsFound { generation: u64, result: Outcome<Vec<SkillDisplay>> },
    NextRandomGroupId { tree: TreeRef, uid: u64, result: Outcome<i32> },
    SaveAnalyzed(Outcome<UsageReport>),
    Committed(Outcome<CommitResult>),
}

pub struct WorkerHandle {
    commands: Sender<Command>,
    events: Receiver<Event>,
}

impl WorkerHandle {
    /// Starts the worker thread. `ctx` is used to wake the UI when an event is ready.
    pub fn spawn(ctx: egui::Context) -> Self {
        let (command_tx, command_rx) = channel::<Command>();
        let (event_tx, event_rx) = channel::<Event>();

        std::thread::Builder::new()
            .name("ysm-worker".into())
            .spawn(move || {
                let mut worker = Worker { repository: None };
                for command in command_rx {
                    let event = worker.handle(command);
                    if event_tx.send(event).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            })
            .expect("the worker thread can be started");

        Self { commands: command_tx, events: event_rx }
    }

    pub fn send(&self, command: Command) {
        // The worker only stops when the application is shutting down.
        let _ = self.commands.send(command);
    }

    pub fn try_recv(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

struct Worker {
    repository: Option<Box<dyn SummonRepository>>,
}

impl Worker {
    fn repository(&mut self) -> Outcome<&mut dyn SummonRepository> {
        match self.repository.as_deref_mut() {
            Some(repository) => Ok(repository),
            None => Err("Database is not configured. Open Settings to set up the connection.".to_owned()),
        }
    }

    /// Runs an operation against the active repository.
    fn with_repository<T>(
        &mut self,
        operation: impl FnOnce(&mut dyn SummonRepository) -> ysm_data::Result<T>,
    ) -> Outcome<T> {
        operation(self.repository()?).map_err(|error| error.to_string())
    }

    fn handle(&mut self, command: Command) -> Event {
        match command {
            Command::Connect(settings) => {
                self.repository = None;
                let result = MssqlRepository::new(settings).and_then(|mut repository| {
                    let version = repository.test_connection()?;
                    self.repository = Some(Box::new(repository));
                    Ok(version)
                });
                Event::Connected(result.map_err(|error| error.to_string()))
            }
            Command::UseDemo => {
                let mut repository = InMemoryRepository::demo();
                let description = repository.test_connection().map_err(|error| error.to_string());
                self.repository = Some(Box::new(repository));
                Event::Connected(description)
            }
            Command::TestConnection(settings) => Event::ConnectionTested(
                MssqlRepository::new(settings)
                    .and_then(|mut repository| repository.test_connection())
                    .map_err(|error| error.to_string()),
            ),
            Command::BuildAssetIndex(path) => Event::AssetIndexBuilt(Arc::new(AssetIndex::build(&path))),
            Command::SearchSummons { filter, generation } => {
                Event::SummonsFound { generation, result: self.with_repository(|r| r.search_summons(&filter)) }
            }
            Command::LoadChain { summon, generation } => {
                let result = self.with_repository(|r| loader::load_chain(r, &summon)).map(Box::new);
                Event::ChainLoaded { generation, summon, result }
            }
            Command::LoadSkillDisplay { ids } => Event::SkillDisplayLoaded(
                self.with_repository(|r| r.skill_display(&ids)).map(|found| found.into_values().collect()),
            ),
            Command::LoadRandomGroups { ids } => {
                let result = self.with_repository(|r| r.random_groups(&ids)).map(|found| found.into_values().collect());
                Event::RandomGroupsLoaded { requested: ids, result }
            }
            Command::LoadSkillRecord { skill_id } => {
                Event::SkillRecordLoaded { skill_id, result: self.with_repository(|r| r.skill_record(skill_id)) }
            }
            Command::LoadScaling { summon_id } => {
                Event::ScalingLoaded(self.with_repository(|r| Ok((r.stat_scaling(summon_id)?, r.creature_enhance()?))))
            }
            Command::SearchSkills { text, generation } => Event::SkillsFound {
                generation,
                result: self.with_repository(|r| r.search_skills(&text, SKILL_SEARCH_LIMIT)),
            },
            Command::NextRandomGroupId { tree, uid } => {
                Event::NextRandomGroupId { tree, uid, result: self.with_repository(|r| r.next_random_group_id()) }
            }
            Command::AnalyzeSave { request, session_tree_ids } => {
                Event::SaveAnalyzed(self.with_repository(|r| loader::analyze_save(r, &request, &session_tree_ids)))
            }
            Command::Commit { request } => Event::Committed(self.with_repository(|r| r.commit(&request))),
        }
    }
}
