//! Multi-query operations built on top of [`SummonRepository`].

use std::collections::{BTreeSet, HashMap};

use ysm_core::commit::CommitRequest;
use ysm_core::evolution::resolve_chain;
use ysm_core::model::{STAGE_COUNT, SkillTreeEntry, SummonListItem, TREE_SLOTS};
use ysm_core::save_plan::UsageReport;
use ysm_core::session::ChainBundle;

use crate::{DataError, Result, SummonRepository};

/// A chain longer than this means `evolve_target` loops.
const MAX_EVOLUTION_HOPS: usize = 8;

/// Follows `evolve_target` to the final evolution of a summon.
///
/// `selected` may be stale (a list row fetched before a save repointed one of
/// its tree slots or its stat row), so it is always read again by ID.
pub fn resolve_final(repo: &mut dyn SummonRepository, selected: &SummonListItem) -> Result<SummonListItem> {
    let mut current = repo.summon_by_id(selected.summon_id)?.unwrap_or_else(|| selected.clone());
    let mut hops = 0;
    while current.evolve_target != 0 {
        hops += 1;
        if hops > MAX_EVOLUTION_HOPS {
            return Err(DataError::Invalid("Evolution chain appears invalid (loop).".into()));
        }
        current = repo
            .summon_by_id(current.evolve_target)?
            .ok_or_else(|| DataError::NotFound(format!("evolve_target {}", current.evolve_target)))?;
    }
    Ok(current)
}

/// Loads the evolution chain of a summon with all its trees, the display
/// data of every referenced skill and the random groups its rows point at.
pub fn load_chain(repo: &mut dyn SummonRepository, selected: &SummonListItem) -> Result<ChainBundle> {
    let final_stage = resolve_final(repo, selected)?;
    let same_card = repo.summons_by_card_id(final_stage.card_id)?;
    let chain = resolve_chain(&final_stage, &same_card)?;

    let mut tree_cache: HashMap<i32, Vec<SkillTreeEntry>> = HashMap::new();
    let mut trees: [[Vec<SkillTreeEntry>; TREE_SLOTS]; STAGE_COUNT] = Default::default();
    for (stage_index, stage_trees) in trees.iter_mut().enumerate() {
        let Some(summon) = chain.stage(stage_index as u8 + 1) else {
            continue;
        };
        for (slot_index, slot) in stage_trees.iter_mut().enumerate() {
            let tree_id = summon.skill_tree_ids[slot_index];
            if tree_id <= 0 {
                continue;
            }
            // A tree used by several slots of the chain is fetched once.
            *slot = match tree_cache.get(&tree_id) {
                Some(entries) => entries.clone(),
                None => {
                    let entries = repo.skill_tree_entries(tree_id)?;
                    tree_cache.insert(tree_id, entries.clone());
                    entries
                }
            };
        }
    }

    let entries = || trees.iter().flatten().flatten();
    let group_ids: Vec<i32> = distinct_positive(entries().map(|e| e.skill_group_id));
    let random_groups = repo.random_groups(&group_ids)?;

    let group_skills = random_groups.values().flat_map(|g| g.skill_ids().iter().copied());
    let skill_ids = distinct_positive(entries().map(|e| e.skill_id).chain(group_skills));
    let skill_display = repo.skill_display(&skill_ids)?;

    Ok(ChainBundle { chain, trees, skill_display, random_groups })
}

/// Collects where the data touched by a save is used.
///
/// `session_tree_ids` are all trees of the edited chain: cloning a random
/// group can pull unchanged trees into the save, and those need the same
/// sharing check.
pub fn analyze_save(
    repo: &mut dyn SummonRepository,
    request: &CommitRequest,
    session_tree_ids: &[i32],
) -> Result<UsageReport> {
    let skill_ids = distinct_positive(request.skills.iter().map(|s| s.skill_id));
    let group_ids = distinct_positive(request.random_groups.iter().map(|g| g.group_id));
    let tree_ids =
        distinct_positive(request.trees.iter().map(|t| t.original_tree_id).chain(session_tree_ids.iter().copied()));
    let stat_ids = distinct_positive(request.stats.iter().map(|s| s.original_stat_id));
    let bonus_ids = distinct_positive(request.level_bonuses.iter().map(|b| b.bonus_id));

    Ok(UsageReport {
        skills: repo.skill_usage(&skill_ids)?,
        random_groups: repo.random_group_usage(&group_ids)?,
        trees: repo.tree_usage(&tree_ids)?,
        stats: repo.stat_usage(&stat_ids)?,
        level_bonuses: repo.level_bonus_usage(&bonus_ids)?,
        next_random_group_id: if group_ids.is_empty() { 0 } else { repo.next_random_group_id()? },
    })
}

fn distinct_positive(ids: impl Iterator<Item = i32>) -> Vec<i32> {
    ids.filter(|id| *id > 0).collect::<BTreeSet<_>>().into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryRepository;
    use ysm_core::model::{EntryField, SummonListFilter};
    use ysm_core::session::{EditSession, TreeRef};

    fn summon(repo: &mut InMemoryRepository, id: i32) -> SummonListItem {
        repo.summon_by_id(id).unwrap().unwrap()
    }

    #[test]
    fn loads_the_whole_chain_from_any_stage() {
        let mut repo = InMemoryRepository::demo();
        let first_stage = summon(&mut repo, 1011);
        let bundle = load_chain(&mut repo, &first_stage).unwrap();

        assert_eq!(bundle.chain.summon_ids(), [1011, 1012, 1013]);
        assert!(bundle.trees.iter().all(|stage| stage.len() == TREE_SLOTS));
        assert!(!bundle.trees[0][0].is_empty());

        let entries: Vec<_> = bundle.trees.iter().flatten().flatten().collect();
        assert!(entries.iter().all(|e| e.skill_id <= 0 || bundle.skill_display.contains_key(&e.skill_id)));
        assert!(entries.iter().any(|e| e.skill_group_id > 0), "demo data has a random group row");
        for group_id in entries.iter().map(|e| e.skill_group_id).filter(|id| *id > 0) {
            let group = &bundle.random_groups[&group_id];
            assert!(group.skill_ids().iter().all(|id| bundle.skill_display.contains_key(id)));
        }
    }

    #[test]
    fn single_stage_summon_loads_as_a_one_stage_chain() {
        let mut repo = InMemoryRepository::demo();
        let lone = summon(&mut repo, 2001);
        let bundle = load_chain(&mut repo, &lone).unwrap();

        assert_eq!(bundle.chain.summon_ids(), [2001]);
        assert!(bundle.trees[0].iter().all(Vec::is_empty));
        assert!(!bundle.trees[2][0].is_empty());
    }

    #[test]
    fn broken_evolve_target_is_reported() {
        let mut repo = InMemoryRepository::demo();
        let mut broken = summon(&mut repo, 1011);
        broken.evolve_target = 999_999;
        repo.insert_summon(broken.clone());

        assert!(matches!(load_chain(&mut repo, &broken), Err(DataError::NotFound(_))));
    }

    #[test]
    fn reloading_with_a_stale_list_row_sees_a_cloned_tree() {
        let mut repo = InMemoryRepository::demo();
        let stale = summon(&mut repo, 1013);
        let mut session = EditSession::new();
        session.load(load_chain(&mut repo, &stale).unwrap());

        let shared = TreeRef::new(3, 4);
        let uid = session.tree(shared).unwrap().rows[0].uid;
        session.set_field(shared, uid, EntryField::Lv, 7);
        let mut request = session.build_commit_request();
        request.trees[0].clone_if_shared = true;
        repo.commit(&request).unwrap();

        // `stale` still carries the old tree ID in slot 4.
        let bundle = load_chain(&mut repo, &stale).unwrap();
        let new_tree_id = bundle.chain.stage3.skill_tree_ids[3];
        assert_ne!(new_tree_id, stale.skill_tree_ids[3]);
        assert!(bundle.trees[2][3].iter().any(|e| e.lv == 7 && e.skill_tree_id == new_tree_id));
    }

    #[test]
    fn analysis_reports_external_use_of_a_shared_tree() {
        let mut repo = InMemoryRepository::demo();
        let selected = summon(&mut repo, 1013);
        let mut session = EditSession::new();
        session.load(load_chain(&mut repo, &selected).unwrap());

        // Slot 4 of the final stage is the tree shared across families.
        let shared = TreeRef::new(3, 4);
        let uid = session.tree(shared).unwrap().rows[0].uid;
        session.set_field(shared, uid, EntryField::JobLv, 99);

        let request = session.build_commit_request();
        let tree_ids: Vec<i32> = session.editable_tree_commits().iter().map(|t| t.original_tree_id).collect();
        let usage = analyze_save(&mut repo, &request, &tree_ids).unwrap();

        let shared_id = session.tree(shared).unwrap().tree_id;
        assert!(usage.trees.iter().any(|u| u.skill_tree_id == shared_id && u.summon_id == 1023));
        assert_eq!(usage.next_random_group_id, 0, "no random group is being saved");
    }

    #[test]
    fn search_finds_summons_by_name_and_id() {
        let mut repo = InMemoryRepository::demo();
        let by_name = repo.search_summons(&SummonListFilter { search_text: "seraph".into(), ..Default::default() });
        assert_eq!(by_name.unwrap().len(), 2);

        let by_id = repo.search_summons(&SummonListFilter { search_text: "1012".into(), ..Default::default() });
        assert_eq!(by_id.unwrap()[0].summon_id, 1012);
    }
}
