//! Summon stat scaling calculator.
//!
//! A summon's stat is its base value plus a growth bonus per evolution stage.
//! Each stage contributes `coefficient × effective stage level × stage
//! multiplier`, truncated; levels past the stage threshold count triple. The
//! creature enhance amplifier is applied to the final sum.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::model::{STAT_COUNT, Stat, StatScalingSnapshot};

pub const MIN_LEVEL: i32 = 1;
pub const MAX_LEVEL: i32 = 350;
pub const MAX_ENHANCE_STAGE: i32 = 5;

struct StageConfig {
    /// Summon level at which this growth stage starts.
    offset: i32,
    /// Stage levels past this count triple.
    threshold: i32,
    max_level: i32,
}

const STAGE_CONFIGS: [StageConfig; 3] = [
    StageConfig { offset: 0, threshold: 50, max_level: 60 },
    StageConfig { offset: 50, threshold: 50, max_level: 65 },
    StageConfig { offset: 100, threshold: 200, max_level: 200 },
];

pub struct ScalingInput<'a> {
    pub level: i32,
    pub enhance_stage: i32,
    /// `CreatureEnhance.stat_amplify` by enhance level.
    pub enhance_amplify: &'a BTreeMap<i32, Decimal>,
    pub snapshot: &'a StatScalingSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalingRow {
    pub stat: Stat,
    pub base: i32,
    /// Growth bonus contributed by each evolution stage.
    pub stage: [i32; 3],
    pub bonus: i32,
    pub final_raw: i32,
    /// `final_raw` with the enhance amplifier applied.
    pub final_value: i32,
    /// Gain over the previous level.
    pub delta: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalingResult {
    pub requested_level: i32,
    pub effective_level: i32,
    pub requested_enhance_stage: i32,
    pub effective_enhance_stage: i32,
    pub stat_amplify: Decimal,
    pub rows: Vec<ScalingRow>,
}

pub fn calculate(input: &ScalingInput<'_>) -> ScalingResult {
    let snapshot = input.snapshot;
    let level = input.level.clamp(MIN_LEVEL, MAX_LEVEL);
    let previous_level = (level - 1).max(0);
    let enhance_stage = input.enhance_stage.clamp(0, MAX_ENHANCE_STAGE);
    let stat_amplify = input.enhance_amplify.get(&enhance_stage).copied().unwrap_or(Decimal::ZERO);
    let multiplier = Decimal::ONE + stat_amplify;
    let growth_depth = snapshot.growth_depth.clamp(1, 3);

    let rows = Stat::ALL
        .into_iter()
        .map(|stat| {
            let mut stage = [0; 3];
            let mut stage_previous = [0; 3];
            for stage_index in 1..=growth_depth {
                let slot = (stage_index - 1) as usize;
                let coefficient = snapshot
                    .stage_bonuses
                    .iter()
                    .find(|b| b.stage_index == stage_index)
                    .map(|b| b.get(stat))
                    .unwrap_or(Decimal::ZERO);
                stage[slot] = stage_bonus(coefficient, stage_index, level);
                stage_previous[slot] = stage_bonus(coefficient, stage_index, previous_level);
            }

            let base = snapshot.base_stats.get(stat);
            let bonus: i32 = stage.iter().sum();
            let bonus_previous: i32 = stage_previous.iter().sum();
            let final_raw = base + bonus;
            let final_value = amplify(final_raw, multiplier);
            let final_previous = amplify(base + bonus_previous, multiplier);

            ScalingRow { stat, base, stage, bonus, final_raw, final_value, delta: final_value - final_previous }
        })
        .collect();

    ScalingResult {
        requested_level: input.level,
        effective_level: level,
        requested_enhance_stage: input.enhance_stage,
        effective_enhance_stage: enhance_stage,
        stat_amplify,
        rows,
    }
}

/// Final stat values for every level `1..=350`, used to plot growth curves.
pub fn growth_curve(
    snapshot: &StatScalingSnapshot,
    enhance_amplify: &BTreeMap<i32, Decimal>,
    enhance_stage: i32,
) -> Vec<[i32; STAT_COUNT]> {
    (MIN_LEVEL..=MAX_LEVEL)
        .map(|level| {
            let result = calculate(&ScalingInput { level, enhance_stage, enhance_amplify, snapshot });
            let mut values = [0; STAT_COUNT];
            for row in result.rows {
                values[row.stat.index()] = row.final_value;
            }
            values
        })
        .collect()
}

fn stage_bonus(coefficient: Decimal, stage_index: i32, level: i32) -> i32 {
    let config = &STAGE_CONFIGS[(stage_index - 1) as usize];
    let stage_level = (level - config.offset).clamp(0, config.max_level);
    if stage_level <= 0 || coefficient.is_zero() {
        return 0;
    }

    let normal_levels = stage_level.min(config.threshold);
    let over_levels = (stage_level - config.threshold).max(0);
    let effective_level = Decimal::from(normal_levels + over_levels * 3);
    let stage_multiplier = Decimal::ONE + Decimal::from(stage_index - 1) * Decimal::new(25, 2);
    truncate(coefficient * effective_level * stage_multiplier)
}

fn amplify(value: i32, multiplier: Decimal) -> i32 {
    truncate(Decimal::from(value) * multiplier)
}

fn truncate(value: Decimal) -> i32 {
    value.trunc().to_i32().unwrap_or(if value.is_sign_negative() { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{LevelBonusStage, StatBlock};

    fn dec(text: &str) -> Decimal {
        text.parse().unwrap()
    }

    fn no_amplify() -> BTreeMap<i32, Decimal> {
        BTreeMap::from([(0, Decimal::ZERO)])
    }

    /// Snapshot where only STR has a base value and growth coefficients.
    fn snapshot(depth: i32, base_str: i32, str_coefficients: &[(i32, &str)]) -> StatScalingSnapshot {
        let mut base = StatBlock::zero(1);
        base.values[0] = base_str;
        StatScalingSnapshot {
            summon_id: 100,
            stat_id: 1,
            evolve_type: depth,
            growth_depth: depth,
            base_stats: base,
            stage_bonuses: str_coefficients
                .iter()
                .map(|(stage_index, coefficient)| {
                    let mut values = [Decimal::ZERO; STAT_COUNT];
                    values[0] = dec(coefficient);
                    LevelBonusStage { stage_index: *stage_index, lookup_id: 100, values, exists_in_db: true }
                })
                .collect(),
        }
    }

    fn str_row(level: i32, enhance: i32, amplify: &BTreeMap<i32, Decimal>, snap: &StatScalingSnapshot) -> ScalingRow {
        let result =
            calculate(&ScalingInput { level, enhance_stage: enhance, enhance_amplify: amplify, snapshot: snap });
        result.rows.into_iter().find(|r| r.stat == Stat::Str).unwrap()
    }

    #[test]
    fn matches_guide_example_for_str() {
        let snap = snapshot(2, 20, &[(1, "0.30"), (2, "0.50")]);
        let row = str_row(80, 0, &no_amplify(), &snap);

        assert_eq!(row.stage[0], 24);
        assert_eq!(row.stage[1], 18);
        assert_eq!(row.bonus, 42);
        assert_eq!(row.final_raw, 62);
        assert_eq!(row.final_value, 62);
    }

    #[test]
    fn truncates_per_stage() {
        let snap = snapshot(1, 0, &[(1, "0.3333")]);
        assert_eq!(str_row(10, 0, &no_amplify(), &snap).bonus, 3);
    }

    #[test]
    fn missing_stage_bonus_counts_as_zero() {
        let snap = snapshot(3, 10, &[(1, "0.2")]);
        let row = str_row(120, 0, &no_amplify(), &snap);

        assert!(row.stage[0] > 0);
        assert_eq!(row.stage[1], 0);
        assert_eq!(row.stage[2], 0);
    }

    #[test]
    fn clamps_level_to_350() {
        let snap = snapshot(1, 0, &[(1, "1")]);
        let result =
            calculate(&ScalingInput { level: 999, enhance_stage: 0, enhance_amplify: &no_amplify(), snapshot: &snap });
        assert_eq!(result.requested_level, 999);
        assert_eq!(result.effective_level, 350);
    }

    #[test]
    fn delta_is_the_gain_over_the_previous_level() {
        let snap = snapshot(1, 0, &[(1, "1")]);
        let level1 = str_row(1, 0, &no_amplify(), &snap);
        let level2 = str_row(2, 0, &no_amplify(), &snap);

        assert_eq!(level1.delta, 1);
        assert_eq!(level2.delta, level2.final_value - level1.final_value);
    }

    #[test]
    fn applies_enhance_amplify_to_the_final_stat() {
        let snap = snapshot(1, 100, &[(1, "0")]);
        let amplify = BTreeMap::from([(0, Decimal::ZERO), (5, dec("0.1234"))]);
        let result =
            calculate(&ScalingInput { level: 1, enhance_stage: 5, enhance_amplify: &amplify, snapshot: &snap });
        let row = result.rows.iter().find(|r| r.stat == Stat::Str).unwrap();

        assert_eq!(row.final_raw, 100);
        assert_eq!(row.final_value, 112);
        assert_eq!(result.effective_enhance_stage, 5);
        assert_eq!(result.stat_amplify, dec("0.1234"));
    }

    #[test]
    fn levels_past_the_threshold_count_triple() {
        // Stage 1: 50 normal levels + 10 over levels × 3 = 80 effective levels.
        let snap = snapshot(1, 0, &[(1, "1")]);
        assert_eq!(str_row(60, 0, &no_amplify(), &snap).bonus, 80);
        // Stage 1 is capped at 60 stage levels.
        assert_eq!(str_row(300, 0, &no_amplify(), &snap).bonus, 80);
    }

    #[test]
    fn growth_curve_covers_every_level_and_never_decreases() {
        let snap = snapshot(3, 5, &[(1, "0.5"), (2, "0.7"), (3, "0.9")]);
        let curve = growth_curve(&snap, &no_amplify(), 0);

        assert_eq!(curve.len(), MAX_LEVEL as usize);
        assert!(curve.windows(2).all(|pair| pair[1][0] >= pair[0][0]));
    }
}
