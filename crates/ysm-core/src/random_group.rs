//! Random skill groups (`SummonRandomSkillResource`).
//!
//! A tree row with `skill_group_id > 0` grants one random skill out of the
//! group's candidates. A group holds at most fifteen distinct skills, kept
//! sorted, and its `use_code` is always the number of candidates.

use std::collections::HashSet;

pub const MAX_GROUP_SKILLS: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomSkillGroup {
    pub group_id: i32,
    skill_ids: Vec<i32>,
}

impl RandomSkillGroup {
    /// Creates a group, normalizing the candidate list.
    pub fn new(group_id: i32, skill_ids: impl IntoIterator<Item = i32>) -> Self {
        Self { group_id, skill_ids: normalize_ids(skill_ids) }
    }

    pub fn empty(group_id: i32) -> Self {
        Self { group_id, skill_ids: Vec::new() }
    }

    /// Reads a database row: only the first `use_code` slots are meaningful.
    pub fn from_db_row(group_id: i32, use_code: i32, slots: [i32; MAX_GROUP_SKILLS]) -> Self {
        let used = use_code.clamp(0, MAX_GROUP_SKILLS as i32) as usize;
        Self::new(group_id, slots.into_iter().take(used))
    }

    pub fn skill_ids(&self) -> &[i32] {
        &self.skill_ids
    }

    pub fn use_code(&self) -> i32 {
        self.skill_ids.len() as i32
    }

    pub fn is_full(&self) -> bool {
        self.skill_ids.len() >= MAX_GROUP_SKILLS
    }

    pub fn contains(&self, skill_id: i32) -> bool {
        self.skill_ids.binary_search(&skill_id).is_ok()
    }

    /// The fifteen `skill_id_N` columns, unused slots zeroed.
    pub fn slots(&self) -> [i32; MAX_GROUP_SKILLS] {
        let mut slots = [0; MAX_GROUP_SKILLS];
        slots[..self.skill_ids.len()].copy_from_slice(&self.skill_ids);
        slots
    }

    /// The same candidates under a different group ID.
    pub fn with_id(&self, group_id: i32) -> Self {
        Self { group_id, skill_ids: self.skill_ids.clone() }
    }
}

/// Positive, distinct, ascending, at most fifteen.
pub fn normalize_ids(ids: impl IntoIterator<Item = i32>) -> Vec<i32> {
    let mut ids: Vec<i32> = ids.into_iter().filter(|id| *id > 0).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.truncate(MAX_GROUP_SKILLS);
    ids
}

/// First group ID at or above the database's next ID that nothing staged uses yet.
pub fn next_free_group_id(db_next: i32, reserved: &HashSet<i32>) -> i32 {
    let mut next = db_next.max(1);
    while reserved.contains(&next) {
        next += 1;
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_candidates() {
        let group = RandomSkillGroup::new(4, [30, 0, 10, -5, 30, 20]);
        assert_eq!(group.skill_ids(), [10, 20, 30]);
        assert_eq!(group.use_code(), 3);
        assert!(group.contains(20));
        assert!(!group.contains(0));
    }

    #[test]
    fn keeps_at_most_fifteen_candidates() {
        let group = RandomSkillGroup::new(1, 1..=40);
        assert_eq!(group.skill_ids().len(), MAX_GROUP_SKILLS);
        assert_eq!(group.skill_ids().last(), Some(&15));
        assert!(group.is_full());
    }

    #[test]
    fn db_row_respects_use_code() {
        let mut slots = [0; MAX_GROUP_SKILLS];
        slots[..4].copy_from_slice(&[50, 40, 30, 20]);

        assert_eq!(RandomSkillGroup::from_db_row(9, 2, slots).skill_ids(), [40, 50]);
        assert_eq!(RandomSkillGroup::from_db_row(9, 0, slots).skill_ids(), [] as [i32; 0]);
        assert_eq!(RandomSkillGroup::from_db_row(9, 99, slots).skill_ids(), [20, 30, 40, 50]);
    }

    #[test]
    fn slots_are_zero_padded() {
        let slots = RandomSkillGroup::new(1, [7, 3]).slots();
        assert_eq!(&slots[..3], [3, 7, 0]);
        assert!(slots[2..].iter().all(|s| *s == 0));
    }

    #[test]
    fn next_free_id_skips_reserved_ids() {
        let reserved = HashSet::from([5, 6, 8]);
        assert_eq!(next_free_group_id(5, &reserved), 7);
        assert_eq!(next_free_group_id(9, &reserved), 9);
        assert_eq!(next_free_group_id(0, &HashSet::new()), 1);
    }
}
