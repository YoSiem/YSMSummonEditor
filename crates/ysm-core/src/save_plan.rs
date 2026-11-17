//! Turns a staged [`CommitRequest`] into the final one by asking the user
//! what to do about data shared with summons outside the edited chain.
//!
//! The planner is a small state machine: read [`SavePlanner::current`], show
//! the question, call [`SavePlanner::answer`], repeat until `current` is
//! `None`, then [`SavePlanner::finish`]. Dropping the planner cancels the save.

use std::collections::{BTreeMap, HashSet, VecDeque};

use crate::commit::{CommitRequest, StatCommit, TreeCommit, TreeKey};
use crate::model::{LevelBonusUsage, RandomGroupUsage, SkillUsage, StatUsage, TreeUsage};
use crate::random_group::{RandomSkillGroup, next_free_group_id};

const MAX_SKILL_ROWS: usize = 15;
const MAX_CHOICE_ROWS: usize = 20;
const MAX_BONUS_ROWS: usize = 30;

/// Where the staged data is used, as reported by the database.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UsageReport {
    pub skills: Vec<SkillUsage>,
    pub random_groups: Vec<RandomGroupUsage>,
    pub trees: Vec<TreeUsage>,
    pub stats: Vec<StatUsage>,
    pub level_bonuses: Vec<LevelBonusUsage>,
    /// Next unused random group ID according to the database.
    pub next_random_group_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveQuestion {
    /// Edited skills are used by other summons. Continue or abort.
    SkillImpact { rows: Vec<SkillUsage> },
    /// A random group is used by other summons. Clone it or edit in place.
    SharedRandomGroup { group_id: i32, rows: Vec<RandomGroupUsage> },
    /// A skill tree is used by other summons. Clone it or edit in place.
    SharedTree { key: TreeKey, rows: Vec<TreeUsage> },
    /// A stat row is used by other summons. Clone it or edit in place.
    SharedStat { summon_id: i32, stat_id: i32, rows: Vec<StatUsage> },
    /// Edited growth bonus rows are used by other summons. Continue or abort.
    LevelBonusImpact { rows: Vec<LevelBonusUsage> },
}

impl SaveQuestion {
    /// Whether the question offers "clone" and "edit in place" (as opposed to just "continue").
    pub fn is_choice(&self) -> bool {
        matches!(
            self,
            SaveQuestion::SharedRandomGroup { .. } | SaveQuestion::SharedTree { .. } | SaveQuestion::SharedStat { .. }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveAnswer {
    /// Accept the impact and go on (for non-choice questions).
    Continue,
    Clone,
    InPlace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Phase {
    Skills,
    RandomGroups,
    Trees,
    Stats,
    LevelBonuses,
    Done,
}

pub struct SavePlanner {
    request: CommitRequest,
    random_groups: BTreeMap<i32, RandomSkillGroup>,
    usage: UsageReport,
    chain_ids: HashSet<i32>,
    /// Every editable tree of the session; used to pull in trees that
    /// reference a cloned random group but were not otherwise changed.
    session_trees: Vec<TreeCommit>,
    reserved_group_ids: HashSet<i32>,
    phase: Phase,
    queue: VecDeque<SaveQuestion>,
    current: Option<SaveQuestion>,
}

impl SavePlanner {
    pub fn new(
        request: CommitRequest,
        usage: UsageReport,
        chain_ids: impl IntoIterator<Item = i32>,
        session_trees: Vec<TreeCommit>,
        reserved_group_ids: HashSet<i32>,
    ) -> Self {
        let random_groups = request.random_groups.iter().map(|g| (g.group_id, g.clone())).collect();
        let mut planner = Self {
            request,
            random_groups,
            usage,
            chain_ids: chain_ids.into_iter().collect(),
            session_trees,
            reserved_group_ids,
            phase: Phase::Skills,
            queue: VecDeque::new(),
            current: None,
        };
        planner.advance();
        planner
    }

    /// The question waiting for an answer, or `None` when planning is complete.
    pub fn current(&self) -> Option<&SaveQuestion> {
        self.current.as_ref()
    }

    pub fn answer(&mut self, answer: SaveAnswer) {
        let Some(question) = self.current.take() else {
            return;
        };
        match question {
            SaveQuestion::SharedRandomGroup { group_id, .. } if answer == SaveAnswer::Clone => {
                self.clone_random_group(group_id);
            }
            SaveQuestion::SharedTree { key, .. } => {
                if let Some(tree) = self.request.trees.iter_mut().find(|t| t.key() == key) {
                    tree.clone_if_shared = answer == SaveAnswer::Clone;
                }
            }
            SaveQuestion::SharedStat { summon_id, stat_id, .. } => {
                for stat in &mut self.request.stats {
                    if stat.summon_id == summon_id && stat.original_stat_id == stat_id {
                        stat.clone_if_shared = answer == SaveAnswer::Clone;
                    }
                }
            }
            _ => {}
        }
        self.advance();
    }

    /// The request to commit. Only meaningful once `current()` is `None`.
    pub fn finish(mut self) -> CommitRequest {
        self.request.random_groups = self.random_groups.into_values().collect();
        self.request
    }

    fn advance(&mut self) {
        while self.queue.is_empty() && self.phase != Phase::Done {
            let questions = match self.phase {
                Phase::Skills => self.skill_questions(),
                Phase::RandomGroups => self.random_group_questions(),
                Phase::Trees => self.tree_questions(),
                Phase::Stats => self.stat_questions(),
                Phase::LevelBonuses => self.level_bonus_questions(),
                Phase::Done => Vec::new(),
            };
            self.queue.extend(questions);
            self.phase = match self.phase {
                Phase::Skills => Phase::RandomGroups,
                Phase::RandomGroups => Phase::Trees,
                Phase::Trees => Phase::Stats,
                Phase::Stats => Phase::LevelBonuses,
                Phase::LevelBonuses | Phase::Done => Phase::Done,
            };
        }
        self.current = self.queue.pop_front();
    }

    fn is_external(&self, summon_id: i32) -> bool {
        !self.chain_ids.contains(&summon_id)
    }

    fn skill_questions(&self) -> Vec<SaveQuestion> {
        let edited: HashSet<i32> = self.request.skills.iter().map(|s| s.skill_id).collect();
        let rows: Vec<SkillUsage> = self
            .usage
            .skills
            .iter()
            .filter(|u| edited.contains(&u.skill_id) && self.is_external(u.summon_id))
            .take(MAX_SKILL_ROWS)
            .cloned()
            .collect();
        if rows.is_empty() { Vec::new() } else { vec![SaveQuestion::SkillImpact { rows }] }
    }

    fn random_group_questions(&self) -> Vec<SaveQuestion> {
        self.random_groups
            .keys()
            .filter_map(|group_id| {
                let rows: Vec<RandomGroupUsage> = self
                    .usage
                    .random_groups
                    .iter()
                    .filter(|u| u.group_id == *group_id && self.is_external(u.summon_id))
                    .take(MAX_CHOICE_ROWS)
                    .cloned()
                    .collect();
                (!rows.is_empty()).then_some(SaveQuestion::SharedRandomGroup { group_id: *group_id, rows })
            })
            .collect()
    }

    fn tree_questions(&self) -> Vec<SaveQuestion> {
        self.request
            .trees
            .iter()
            .filter_map(|tree| {
                let mut rows: Vec<TreeUsage> = self
                    .usage
                    .trees
                    .iter()
                    .filter(|u| u.skill_tree_id == tree.original_tree_id && self.is_external(u.summon_id))
                    .cloned()
                    .collect();
                rows.sort_by_key(|u| (u.summon_id, u.tree_slot));
                rows.truncate(MAX_CHOICE_ROWS);
                (!rows.is_empty()).then_some(SaveQuestion::SharedTree { key: tree.key(), rows })
            })
            .collect()
    }

    fn stat_questions(&self) -> Vec<SaveQuestion> {
        self.request
            .stats
            .iter()
            .filter_map(|stat: &StatCommit| {
                let rows: Vec<StatUsage> = self
                    .usage
                    .stats
                    .iter()
                    .filter(|u| u.stat_id == stat.original_stat_id && self.is_external(u.summon_id))
                    .take(MAX_CHOICE_ROWS)
                    .cloned()
                    .collect();
                (!rows.is_empty()).then_some(SaveQuestion::SharedStat {
                    summon_id: stat.summon_id,
                    stat_id: stat.original_stat_id,
                    rows,
                })
            })
            .collect()
    }

    fn level_bonus_questions(&self) -> Vec<SaveQuestion> {
        let edited: HashSet<i32> = self.request.level_bonuses.iter().map(|b| b.bonus_id).collect();
        let rows: Vec<LevelBonusUsage> = self
            .usage
            .level_bonuses
            .iter()
            .filter(|u| edited.contains(&u.bonus_id) && self.is_external(u.summon_id))
            .take(MAX_BONUS_ROWS)
            .cloned()
            .collect();
        if rows.is_empty() { Vec::new() } else { vec![SaveQuestion::LevelBonusImpact { rows }] }
    }

    /// Gives the staged version of a group a fresh ID and repoints every row
    /// of the edited chain at it, leaving the shared original untouched.
    fn clone_random_group(&mut self, group_id: i32) {
        let Some(group) = self.random_groups.remove(&group_id) else {
            return;
        };

        // Trees that reference the group must be rewritten even if nothing else in them changed.
        for tree in &self.session_trees {
            let references_group = tree.entries.iter().any(|e| e.skill_group_id == group_id);
            if references_group && !self.request.trees.iter().any(|t| t.key() == tree.key()) {
                self.request.trees.push(tree.clone());
            }
        }

        let new_id = next_free_group_id(self.usage.next_random_group_id, &self.reserved_group_ids);
        self.reserved_group_ids.insert(new_id);
        self.random_groups.insert(new_id, group.with_id(new_id));

        for entry in self.request.trees.iter_mut().flat_map(|t| &mut t.entries) {
            if entry.skill_group_id == group_id {
                entry.skill_group_id = new_id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit::{FieldChange, LevelBonusUpsert, SkillUpdate};
    use crate::model::{EntryField, StatBlock};
    use crate::session::TreeRef;
    use crate::session::test_support::{first_uid, loaded};
    use crate::value::{FieldKind, FieldValue};

    const CHAIN: [i32; 3] = [1, 2, 3];
    const T11: TreeRef = TreeRef { stage: 1, slot: 1 };
    const T31: TreeRef = TreeRef { stage: 3, slot: 1 };

    fn tree_usage(tree_id: i32, summon_id: i32, slot: i32) -> TreeUsage {
        TreeUsage { skill_tree_id: tree_id, summon_id, summon_name: format!("S{summon_id}"), tree_slot: slot }
    }

    fn group_usage(group_id: i32, summon_id: i32) -> RandomGroupUsage {
        RandomGroupUsage { group_id, skill_tree_id: 999, summon_id, summon_name: format!("S{summon_id}"), tree_slot: 1 }
    }

    fn planner(session: &crate::session::EditSession, usage: UsageReport) -> SavePlanner {
        SavePlanner::new(
            session.build_commit_request(),
            usage,
            CHAIN,
            session.editable_tree_commits(),
            session.reserved_group_ids(),
        )
    }

    #[test]
    fn usage_inside_the_chain_raises_no_questions() {
        let mut session = loaded();
        let uid = first_uid(&session, T11);
        session.set_field(T11, uid, EntryField::Lv, 9);
        let usage = UsageReport { trees: vec![tree_usage(10, 1, 1), tree_usage(10, 2, 3)], ..Default::default() };

        let plan = planner(&session, usage);
        assert_eq!(plan.current(), None);
        let request = plan.finish();
        assert_eq!(request.trees.len(), 1);
        assert!(!request.trees[0].clone_if_shared);
    }

    #[test]
    fn shared_tree_can_be_cloned_or_edited_in_place() {
        let mut session = loaded();
        let uid = first_uid(&session, T11);
        session.set_field(T11, uid, EntryField::Lv, 9);
        let usage = UsageReport {
            trees: vec![tree_usage(10, 1, 1), tree_usage(10, 88, 2), tree_usage(10, 77, 1)],
            ..Default::default()
        };

        let mut plan = planner(&session, usage.clone());
        let Some(SaveQuestion::SharedTree { key, rows }) = plan.current().cloned() else {
            panic!("expected a shared tree question, got {:?}", plan.current());
        };
        assert_eq!(key, TreeKey { summon_id: 1, slot: 1, tree_id: 10 });
        assert_eq!(rows.iter().map(|r| r.summon_id).collect::<Vec<_>>(), [77, 88], "external only, sorted");
        assert!(plan.current().unwrap().is_choice());

        plan.answer(SaveAnswer::Clone);
        assert_eq!(plan.current(), None);
        assert!(plan.finish().trees[0].clone_if_shared);

        let mut in_place = planner(&session, usage);
        in_place.answer(SaveAnswer::InPlace);
        assert!(!in_place.finish().trees[0].clone_if_shared);
    }

    #[test]
    fn cloning_a_random_group_remaps_rows_and_pulls_in_unchanged_trees() {
        let mut session = loaded();
        session.stage_random_group(RandomSkillGroup::new(7, [900, 100]));
        assert!(session.build_commit_request().trees.is_empty(), "the tree itself is unchanged");

        let usage = UsageReport {
            random_groups: vec![group_usage(7, 3), group_usage(7, 55)],
            next_random_group_id: 7,
            ..Default::default()
        };
        let mut plan = planner(&session, usage);
        let Some(SaveQuestion::SharedRandomGroup { group_id, rows }) = plan.current().cloned() else {
            panic!("expected a shared group question");
        };
        assert_eq!(group_id, 7);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].summon_id, 55);

        plan.answer(SaveAnswer::Clone);
        assert_eq!(plan.current(), None);
        let request = plan.finish();

        assert_eq!(request.random_groups.len(), 1);
        let cloned = &request.random_groups[0];
        assert_eq!(cloned.group_id, 8, "7 is reserved by the session");
        assert_eq!(cloned.skill_ids(), [100, 900]);

        assert_eq!(request.trees.len(), 1);
        assert_eq!(request.trees[0].original_tree_id, 30);
        assert_eq!(request.trees[0].entries[0].skill_group_id, 8);
    }

    #[test]
    fn editing_a_shared_random_group_in_place_keeps_its_id() {
        let mut session = loaded();
        session.stage_random_group(RandomSkillGroup::new(7, [900, 100]));
        let usage =
            UsageReport { random_groups: vec![group_usage(7, 55)], next_random_group_id: 8, ..Default::default() };

        let mut plan = planner(&session, usage);
        plan.answer(SaveAnswer::InPlace);
        let request = plan.finish();

        assert_eq!(request.random_groups[0].group_id, 7);
        assert!(request.trees.is_empty());
    }

    #[test]
    fn a_tree_pulled_in_by_a_group_clone_is_checked_for_sharing_afterwards() {
        let mut session = loaded();
        session.stage_random_group(RandomSkillGroup::new(7, [900, 100]));
        let usage = UsageReport {
            random_groups: vec![group_usage(7, 55)],
            trees: vec![tree_usage(30, 55, 1)],
            next_random_group_id: 20,
            ..Default::default()
        };

        let mut plan = planner(&session, usage);
        plan.answer(SaveAnswer::Clone);
        assert!(matches!(plan.current(), Some(SaveQuestion::SharedTree { key, .. }) if key.tree_id == 30));
        plan.answer(SaveAnswer::Clone);

        let request = plan.finish();
        assert!(request.trees[0].clone_if_shared);
        assert_eq!(request.trees[0].entries[0].skill_group_id, 20);
    }

    #[test]
    fn questions_follow_the_original_order() {
        let mut session = loaded();
        let uid = first_uid(&session, T31);
        session.set_field(T31, uid, EntryField::Lv, 4);
        session.stage_skill_update(SkillUpdate {
            skill_id: 300,
            changes: vec![FieldChange {
                name: "var1".into(),
                kind: FieldKind::I32,
                old: FieldValue::I32(1),
                new: FieldValue::I32(2),
            }],
        });

        let mut request = session.build_commit_request();
        request.stats.push(StatCommit {
            summon_id: 3,
            original_stat_id: 90,
            stats: StatBlock::zero(90),
            clone_if_shared: false,
        });
        request.level_bonuses.push(LevelBonusUpsert { bonus_id: 3, values: Default::default() });

        let usage = UsageReport {
            skills: vec![SkillUsage { skill_id: 300, skill_tree_id: 1, summon_id: 60, summon_name: "X".into() }],
            trees: vec![tree_usage(30, 60, 1)],
            stats: vec![StatUsage { stat_id: 90, summon_id: 60, summon_name: "X".into() }],
            level_bonuses: vec![LevelBonusUsage {
                bonus_id: 3,
                summon_id: 60,
                summon_name: "X".into(),
                stage_index: 1,
            }],
            ..Default::default()
        };
        let mut plan =
            SavePlanner::new(request, usage, CHAIN, session.editable_tree_commits(), session.reserved_group_ids());

        assert!(matches!(plan.current(), Some(SaveQuestion::SkillImpact { .. })));
        assert!(!plan.current().unwrap().is_choice());
        plan.answer(SaveAnswer::Continue);
        assert!(matches!(plan.current(), Some(SaveQuestion::SharedTree { .. })));
        plan.answer(SaveAnswer::InPlace);
        assert!(matches!(plan.current(), Some(SaveQuestion::SharedStat { stat_id: 90, .. })));
        plan.answer(SaveAnswer::Clone);
        assert!(matches!(plan.current(), Some(SaveQuestion::LevelBonusImpact { .. })));
        plan.answer(SaveAnswer::Continue);
        assert_eq!(plan.current(), None);

        let request = plan.finish();
        assert!(request.stats[0].clone_if_shared);
        assert!(!request.trees[0].clone_if_shared);
    }
}
