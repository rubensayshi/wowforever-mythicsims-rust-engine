//! Forever's rules for a target the reference's tables have no column for (docs/level-gap.md).
//!
//! The reference's attack table has a column for a target at the character's level, 1, 2 and 3
//! above it and 2 below it; any other level reads the 3 above column (`unit_level_f64`), so a
//! level 55 dungeon mob dodged, glanced and resisted like a raid boss. Under Forever's rules a
//! target outside those columns reads the rules here, by the gap between its level and the
//! character's:
//!
//! - Below the character, a player's miss, dodge and parry fall 0.5% a level to nothing 10 levels
//!   down, the line the 2 below column is already on (0.1% a point of defense under the attack's
//!   skill). No swing glances, nothing is taken off hit or crit, and a spell misses 1% less a
//!   level.
//! - From 4 above, miss and dodge follow the weapon skill formulas at base skill (5% and 0.2% a
//!   point of defense over skill, 5% and 0.1% a point), which is where the 3 above column already
//!   sits; a glancing blow is 10% likelier a level and keeps what the weapon skill formula says;
//!   crit loses 1% a level more; a spell misses 11% more a level.
//! - An enemy's swings at the player stay on the line their columns are on, 0.2% a level, and a
//!   crushing blow is 10% likelier a level from 3 above.
//!
//! Each rule's source is in docs/level-gap.md: Blizzard's own statements for the 3 above column
//! and the per level dodge, crit and spell hit rules, and public Forever combat logs for the rest
//! (305 dungeon reports, 23 November 2026). The rules that neither covers are named there as
//! approximations: parry outside the columns, the crushing blow past 3 above and the 1.8% of crit
//! the 3 above column takes beyond its 3 levels.
//!
//! A gap the reference has a column for never reaches this module, so every value a level 60 to
//! 63 target or a levelling character's 0 to 3 above target reads is the reference's own.

use super::attack::AttackTable;
use super::levels::character_level;
use super::sim::UnitType;
use super::weapon_skill;

/// How far above the character Forever's rules follow a target: public logs carry the glancing
/// chance to 5 above (58% of 416 swings for the rule's 60%) and leave it at 6 (62% for 70%).
pub(crate) const MAX_LEVELS_ABOVE: i32 = 5;

/// The levels a unit is above the character being prepared; negative below it.
pub(crate) fn gap(level: i32) -> i32 {
    level - character_level()
}

/// Whether the reference's tables have a column for a unit this many levels above the character.
pub(crate) fn has_column(gap: i32) -> bool {
    gap == -2 || (0..=3).contains(&gap)
}

/// A player's chance to miss, be dodged or be parried by an enemy below it: 5% less 0.5% a
/// level, none from 10 below.
fn below(gap: i32) -> f64 {
    f64::from(gap).mul_add(0.005, 0.05).max(0.0)
}

/// A spell's chance to miss an enemy: 4% at the caster's level, 1% a level either way to 2
/// above and to nothing 4 below, then 17% at 3 above and 11% a level more (Blizzard's table:
/// 83%, 72% and 61% to hit at 3, 4 and 5 above).
pub(crate) fn spell_miss(gap: i32) -> f64 {
    if gap <= 2 {
        f64::from(gap).mul_add(0.01, 0.04).max(0.0)
    } else {
        f64::from(gap - 3).mul_add(0.11, 0.17).min(0.99)
    }
}

/// The share of swings that glance off an enemy: none below the attacker, 10% at its level and
/// 10% a level more.
pub(crate) fn glance_chance(gap: i32) -> f64 {
    f64::from(gap).mul_add(0.10, 0.10).clamp(0.0, 1.0)
}

/// What an enemy above the attacker takes off its melee crit: 1% a level, and from 3 above the
/// 1.8% the reference's boss column adds.
pub(crate) fn melee_crit_suppression(gap: i32) -> f64 {
    if gap <= 0 {
        0.0
    } else if gap < 3 {
        f64::from(gap) * 0.01
    } else {
        f64::from(gap).mul_add(0.01, 0.018)
    }
}

/// An enemy's chance to parry from 3 above: 5% and 0.6% a point of defense over skill, the 14%
/// of the boss column. An approximation past it.
fn parry_above(gap: i32) -> f64 {
    f64::from(5 * gap).mul_add(0.006, 0.05)
}

/// An enemy's base crit against the player, in percent: 5% and 0.2% a level it is above.
pub(crate) fn enemy_crit_percent(gap: i32) -> f64 {
    f64::from(gap).mul_add(0.2, 5.0).max(0.0)
}

/// An enemy's chance to land a crushing blow: 2% a point its skill is over the player's
/// defense less 15%, none under 3 levels above. An approximation past 3 above.
pub(crate) fn crush_chance(gap: i32) -> f64 {
    if gap < 3 {
        0.0
    } else {
        f64::from(gap).mul_add(0.10, -0.15).min(1.0)
    }
}

/// Moves a table the reference built to Forever's rules when no reference column covers its
/// gap: an enemy defender by its own level, a player defender by the attacking enemy's.
pub(crate) fn apply(
    table: &mut AttackTable,
    attacker_level: i32,
    defender_type: UnitType,
    defender_level: i32,
) {
    if defender_type == UnitType::Enemy {
        let gap = gap(defender_level);
        if has_column(gap) {
            return;
        }
        table.base_spell_miss_chance = spell_miss(gap);
        table.base_glance_chance = glance_chance(gap);
        table.melee_crit_suppression = melee_crit_suppression(gap);
        if gap < 0 {
            let avoided = below(gap);
            table.base_miss_chance = avoided;
            table.base_dodge_chance = avoided;
            table.base_parry_chance = avoided;
            // No swing glances; the fields keep what a glance at the attacker's level reads.
            table.glance_multiplier = 0.95;
            table.glance_spread = 0.04;
            table.hit_suppression = 0.0;
        } else {
            let skill_gap = f64::from(5 * gap);
            table.base_miss_chance = weapon_skill::avoidance(skill_gap, 0.002);
            table.base_dodge_chance = weapon_skill::avoidance(skill_gap, 0.001);
            table.base_parry_chance = parry_above(gap);
            (table.glance_multiplier, table.glance_spread) = weapon_skill::glance_at(skill_gap);
            // Blizzard: the first 1% of hit is ignored past a 10 point gap, whatever the gap.
            table.hit_suppression = 0.01;
        }
    } else {
        let gap = gap(attacker_level);
        if has_column(gap) {
            return;
        }
        let avoided = f64::from(gap).mul_add(-0.002, 0.05).max(0.0);
        table.base_miss_chance = avoided;
        table.base_block_chance = avoided;
        table.base_parry_chance = avoided;
        table.base_dodge_chance = f64::from(gap) * -0.002;
        table.base_crush_chance = crush_chance(gap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prepare::attack::new_attack_table;

    const LEVEL: i32 = 60;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    /// The table a level 60 character reads against an enemy `gap` levels above it under
    /// Forever's rules.
    fn against(gap: i32) -> AttackTable {
        assert_eq!(character_level(), LEVEL);
        let mut table = new_attack_table(LEVEL, UnitType::Enemy, LEVEL + gap);
        apply(&mut table, LEVEL, UnitType::Enemy, LEVEL + gap);
        table
    }

    /// The table an enemy `gap` levels above a level 60 character reads against it.
    fn from_enemy(gap: i32) -> AttackTable {
        let mut table = new_attack_table(LEVEL + gap, UnitType::Player, LEVEL);
        apply(&mut table, LEVEL + gap, UnitType::Player, LEVEL);
        table
    }

    fn bits(table: &AttackTable) -> [u64; 12] {
        [
            table.base_miss_chance,
            table.base_spell_miss_chance,
            table.base_block_chance,
            table.base_dodge_chance,
            table.base_parry_chance,
            table.base_glance_chance,
            table.base_crush_chance,
            table.glance_multiplier,
            table.glance_spread,
            table.melee_crit_suppression,
            table.spell_crit_suppression,
            table.hit_suppression,
        ]
        .map(f64::to_bits)
    }

    /// The values a target at the character's level to 3 above reads, as literals: a change to
    /// any of them moves every board.
    #[test]
    fn the_reference_columns_are_pinned() {
        // spell miss, miss, dodge, parry, glance chance, glance multiplier, glance spread, hit
        // suppression, crit suppression.
        for (gap, want) in [
            (0, [0.04, 0.05, 0.05, 0.05, 0.10, 0.95, 0.04, 0.0, 0.0]),
            (1, [0.05, 0.055, 0.055, 0.055, 0.20, 0.95, 0.04, 0.0, 0.01]),
            (2, [0.06, 0.06, 0.06, 0.06, 0.30, 0.85, 0.05, 0.0, 0.02]),
            (3, [0.17, 0.08, 0.065, 0.14, 0.40, 0.65, 0.10, 0.01, 0.048]),
        ] {
            let table = against(gap);
            let got = [
                table.base_spell_miss_chance,
                table.base_miss_chance,
                table.base_dodge_chance,
                table.base_parry_chance,
                table.base_glance_chance,
                table.glance_multiplier,
                table.glance_spread,
                table.hit_suppression,
                table.melee_crit_suppression,
            ];
            assert_eq!(got.map(f64::to_bits), want.map(f64::to_bits), "{gap} above");
            assert_eq!(table.base_block_chance.to_bits(), 0.05f64.to_bits());
        }
        // An enemy's swings at the player: miss, block, dodge, parry, crush.
        for (gap, want) in [
            (0, [0.05, 0.05, 0.0, 0.05, 0.0]),
            (1, [0.048, 0.048, -0.002, 0.048, 0.0]),
            (2, [0.046, 0.046, -0.004, 0.046, 0.0]),
            (3, [0.044, 0.044, -0.006, 0.044, 0.15]),
        ] {
            let table = from_enemy(gap);
            let got = [
                table.base_miss_chance,
                table.base_block_chance,
                table.base_dodge_chance,
                table.base_parry_chance,
                table.base_crush_chance,
            ];
            assert_eq!(got.map(f64::to_bits), want.map(f64::to_bits), "{gap} above");
            assert_eq!(table.base_spell_miss_chance.to_bits(), 0.05f64.to_bits());
        }
    }

    /// Where the reference has a column, the rules leave its table alone, bit for bit.
    #[test]
    fn a_reference_column_is_never_touched() {
        for gap in [-2, 0, 1, 2, 3] {
            assert!(has_column(gap));
            for defender in [UnitType::Enemy, UnitType::Player] {
                let (attacker, target) = if defender == UnitType::Enemy {
                    (LEVEL, LEVEL + gap)
                } else {
                    (LEVEL + gap, LEVEL)
                };
                let reference = new_attack_table(attacker, defender, target);
                let mut table = reference.clone();
                apply(&mut table, attacker, defender, target);
                assert_eq!(bits(&table), bits(&reference), "{gap} above, {defender:?}");
            }
        }
        for gap in [-30, -5, -3, -1, 4, 5, 8] {
            assert!(!has_column(gap));
        }
    }

    /// Each rule meets the column on either side of it, so the rules are the columns' own lines.
    #[test]
    fn the_rules_meet_the_columns() {
        for gap in [-2, 0, 1, 2, 3] {
            let table = new_attack_table(LEVEL, UnitType::Enemy, LEVEL + gap);
            assert!(near(spell_miss(gap), table.base_spell_miss_chance), "{gap}");
            assert!(near(glance_chance(gap), table.base_glance_chance), "{gap}");
            assert!(
                near(melee_crit_suppression(gap), table.melee_crit_suppression),
                "{gap}"
            );
            let enemy = new_attack_table(LEVEL + gap, UnitType::Player, LEVEL);
            assert!(near(crush_chance(gap), enemy.base_crush_chance), "{gap}");
            assert!(
                near(f64::from(gap).mul_add(-0.002, 0.05), enemy.base_miss_chance),
                "{gap}"
            );
            assert!(
                near(f64::from(gap) * -0.002, enemy.base_dodge_chance),
                "{gap}"
            );
        }
        let two_below = new_attack_table(LEVEL, UnitType::Enemy, LEVEL - 2);
        assert!(near(below(-2), two_below.base_miss_chance));
        assert!(near(below(-2), two_below.base_dodge_chance));
        assert!(near(below(-2), two_below.base_parry_chance));
        let boss = new_attack_table(LEVEL, UnitType::Enemy, LEVEL + 3);
        assert!(near(
            weapon_skill::avoidance(15.0, 0.002),
            boss.base_miss_chance
        ));
        assert!(near(
            weapon_skill::avoidance(15.0, 0.001),
            boss.base_dodge_chance
        ));
        assert!(near(parry_above(3), boss.base_parry_chance));
        let (multiplier, spread) = weapon_skill::glance_at(15.0);
        assert!(near(multiplier, boss.glance_multiplier));
        assert!(near(spread, boss.glance_spread));
    }

    /// An enemy below the character: miss, dodge and parry fall 0.5% a level to nothing 10
    /// below, no swing glances, crit and hit lose nothing and a spell misses 1% less a level.
    #[test]
    fn an_enemy_below_the_character() {
        // gap, miss and dodge and parry, spell miss.
        for (gap, avoided, spell) in [
            (-1, 0.045, 0.03),
            (-3, 0.035, 0.01),
            (-5, 0.025, 0.0),
            (-9, 0.005, 0.0),
            (-10, 0.0, 0.0),
            (-30, 0.0, 0.0),
            (-50, 0.0, 0.0),
        ] {
            let table = against(gap);
            assert!(near(table.base_miss_chance, avoided), "{gap}");
            assert!(near(table.base_dodge_chance, avoided), "{gap}");
            assert!(near(table.base_parry_chance, avoided), "{gap}");
            assert!(near(table.base_spell_miss_chance, spell), "{gap}");
            assert_eq!(table.base_block_chance, 0.05, "{gap}");
            assert_eq!(table.base_glance_chance, 0.0, "{gap}");
            assert_eq!(table.hit_suppression, 0.0, "{gap}");
            assert_eq!(table.melee_crit_suppression, 0.0, "{gap}");
        }
    }

    /// An enemy 4 and 5 above the character, and past the range the gate lets through.
    #[test]
    fn an_enemy_past_three_above() {
        // gap, miss, dodge, parry, glance chance, glance multiplier, glance spread, crit
        // suppression, spell miss.
        for (gap, want) in [
            (4, [0.09, 0.07, 0.17, 0.50, 0.45, 0.15, 0.058, 0.28]),
            (5, [0.10, 0.075, 0.20, 0.60, 0.25, 0.20, 0.068, 0.39]),
            (8, [0.13, 0.09, 0.29, 0.90, 0.105, 0.095, 0.098, 0.72]),
        ] {
            let table = against(gap);
            let got = [
                table.base_miss_chance,
                table.base_dodge_chance,
                table.base_parry_chance,
                table.base_glance_chance,
                table.glance_multiplier,
                table.glance_spread,
                table.melee_crit_suppression,
                table.base_spell_miss_chance,
            ];
            for (got, want) in got.iter().zip(want) {
                assert!(near(*got, want), "{gap} above: {got} for {want}");
            }
            assert_eq!(table.hit_suppression, 0.01, "{gap}");
            assert_eq!(table.base_block_chance, 0.05, "{gap}");
        }
        assert_eq!(against(20).base_glance_chance, 1.0);
        assert_eq!(against(20).base_spell_miss_chance, 0.99);
    }

    /// An enemy's swings at the player outside the columns stay on their line: 0.2% a level,
    /// floored at nothing, with a crushing blow 10% likelier a level from 3 above.
    #[test]
    fn an_enemy_swinging_at_the_player() {
        // gap, miss and block and parry, dodge, crush.
        for (gap, avoided, dodge, crush) in [
            (-5, 0.06, 0.01, 0.0),
            (-1, 0.052, 0.002, 0.0),
            (4, 0.042, -0.008, 0.25),
            (5, 0.04, -0.01, 0.35),
            (8, 0.034, -0.016, 0.65),
            (30, 0.0, -0.06, 1.0),
        ] {
            let table = from_enemy(gap);
            assert!(near(table.base_miss_chance, avoided), "{gap}");
            assert!(near(table.base_block_chance, avoided), "{gap}");
            assert!(near(table.base_parry_chance, avoided), "{gap}");
            assert!(near(table.base_dodge_chance, dodge), "{gap}");
            assert!(near(table.base_crush_chance, crush), "{gap}");
            assert_eq!(table.base_spell_miss_chance, 0.05, "{gap}");
        }
    }

    #[test]
    fn an_enemys_crit_follows_its_level() {
        // The columns: 4.6, 5.0, 5.2, 5.4 and 5.6 at 2 below and 0 to 3 above.
        for (gap, column) in [(-2, 4.6), (0, 5.0), (1, 5.2), (2, 5.4), (3, 5.6)] {
            assert!(near(enemy_crit_percent(gap), column), "{gap}");
        }
        for (gap, want) in [
            (-5, 4.0),
            (-1, 4.8),
            (4, 5.8),
            (5, 6.0),
            (8, 6.6),
            (-25, 0.0),
        ] {
            assert!(near(enemy_crit_percent(gap), want), "{gap}");
        }
        assert_eq!(enemy_crit_percent(-40), 0.0);
    }
}
