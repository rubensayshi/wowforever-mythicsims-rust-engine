//! A target of any level (docs/level-gap.md, 23 November 2026).
//!
//! Until now a character fought a target from its own level to 3 above, the levels the
//! reference's attack tables have a column for. Under Forever's rules it fights one from level
//! 10 to 5 above its own, and a target outside the columns reads tables that follow the gap
//! between the two levels (`prepare::level_gap`). The level 60 fixtures are board requests of
//! three reference builds (fixtures/level-gap); the levelling ones are docs/levels.md's.

use forever_engine::contracts::prepared_v2::PreparedV2;
use forever_engine::{prepare_json_with, prepared_refusals, simulate_prepared, Rules};
use serde_json::Value;
use std::{fs, path::Path};

fn request(path: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("fixtures/{path}.request.json"));
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

/// The request with its boss at `level`.
fn against(request: &Value, level: i64) -> Value {
    let mut request = request.clone();
    for target in request["encounter"]["targets"].as_array_mut().unwrap() {
        target["level"] = level.into();
    }
    request
}

fn prepare_with(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules).unwrap()
}

fn prepare(request: &Value) -> Value {
    prepare_with(request, Rules::Forever)
}

/// The gate's refusals of a prepared document, as code and reason.
fn refusals(prepared: &Value) -> Vec<(String, String)> {
    let typed: PreparedV2 = serde_json::from_value(prepared.clone()).unwrap();
    prepared_refusals(&typed)
        .into_iter()
        .map(|refusal| (refusal.code.to_string(), refusal.reason))
        .collect()
}

fn passes(prepared: &Value) {
    let refused = refusals(prepared);
    assert!(refused.is_empty(), "gate refusals: {refused:?}");
}

/// The player's metrics of a prepared document the gate passes.
fn simulate(prepared: &Value) -> Value {
    passes(prepared);
    let typed: PreparedV2 = serde_json::from_value(prepared.clone()).unwrap();
    let report = simulate_prepared(&typed).unwrap();
    report.result["raidMetrics"]["parties"][0]["players"][0].clone()
}

fn dps(player: &Value) -> f64 {
    player["dps"]["avg"].as_f64().unwrap()
}

/// The white swings of a player's metrics by outcome: all of them and those that glanced.
fn white_swings(player: &Value) -> (f64, f64) {
    let (mut swings, mut glances) = (0.0, 0.0);
    for action in player["actions"].as_array().unwrap() {
        // Tags 1 and 2 are the main hand's and the off hand's swings; other tags are extra
        // attacks on the special table, which never glance.
        let id = &action["id"];
        if id["otherId"] != "OtherActionAttack" || !(id["tag"] == 1 || id["tag"] == 2) {
            continue;
        }
        for target in action["targets"].as_array().unwrap() {
            for outcome in [
                "hits", "crits", "glances", "misses", "dodges", "parries", "blocks",
            ] {
                swings += target[outcome].as_f64().unwrap_or(0.0);
            }
            glances += target["glances"].as_f64().unwrap_or(0.0);
        }
    }
    (swings, glances)
}

fn f(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("a number: {value}"))
}

fn near(got: f64, want: f64) -> bool {
    (got - want).abs() < 1e-9
}

#[test]
fn a_level_60_character_fights_level_10_to_65() {
    let rogue = request("level-gap/combat-rogue");
    for level in [10, 30, 55, 59, 60, 61, 62, 63, 64, 65] {
        let prepared = prepare(&against(&rogue, level));
        assert_eq!(prepared["target"]["level"], level);
        assert!(dps(&simulate(&prepared)) > 0.0, "level {level}");
    }
    for level in [9, 66, 70] {
        let prepared = prepare(&against(&rogue, level));
        assert_eq!(
            refusals(&prepared),
            [(
                "level_unsupported".to_string(),
                format!("target level {level} is not supported")
            )]
        );
    }
}

/// A target at the character's level to 3 above prepares as it always did, without the mark; any
/// other level carries it, and the gate refuses that level without it, as it refuses a reference
/// document.
#[test]
fn only_a_level_outside_the_columns_carries_the_mark() {
    let rogue = request("level-gap/combat-rogue");
    for level in 60..=63 {
        let prepared = prepare(&against(&rogue, level));
        assert!(prepared["target"].get("level_gap_tables").is_none());
        passes(&prepared);
    }
    for level in [10, 58, 59, 64, 65] {
        let mut prepared = prepare(&against(&rogue, level));
        assert_eq!(prepared["target"]["level_gap_tables"], true);
        passes(&prepared);
        prepared["target"]
            .as_object_mut()
            .unwrap()
            .remove("level_gap_tables");
        assert_eq!(
            refusals(&prepared),
            [(
                "level_unsupported".to_string(),
                format!("target level {level} is not supported")
            )]
        );
    }
}

/// The reference gives a target it has no column for the 3 above column and no mark, so the
/// gate still refuses it under the reference's rules; Forever's rules follow the gap.
#[test]
fn the_reference_rules_keep_their_columns() {
    let mage = against(&request("levels/frost-mage-L20"), 25);
    let reference = prepare_with(&mage, Rules::GoReference);
    assert!(reference["target"].get("level_gap_tables").is_none());
    assert_eq!(
        f(&reference["player"]["attack_table"]["base_spell_miss_chance"]),
        0.17
    );
    assert_eq!(f(&reference["target"]["stats"]["PhysicalCritPercent"]), 5.6);
    assert_eq!(
        refusals(&reference),
        [(
            "level_unsupported".to_string(),
            "target level 25 is not supported".to_string()
        )]
    );
    let forever = prepare(&mage);
    assert!(near(
        f(&forever["player"]["attack_table"]["base_spell_miss_chance"]),
        0.39
    ));
    assert!(near(
        f(&forever["target"]["stats"]["PhysicalCritPercent"]),
        6.0
    ));
    passes(&forever);
}

#[test]
fn the_melee_table_follows_the_gap() {
    let rogue = request("level-gap/combat-rogue");
    // level, miss, glance chance, glance multiplier, glance spread, crit suppression.
    for (level, want) in [
        (10, [0.0, 0.0, 0.95, 0.04, 0.0]),
        (55, [0.025, 0.0, 0.95, 0.04, 0.0]),
        (59, [0.045, 0.0, 0.95, 0.04, 0.0]),
        // The reference's columns, unchanged.
        (60, [0.05, 0.10, 0.95, 0.04, 0.0]),
        (61, [0.055, 0.20, 0.95, 0.04, 0.01]),
        (62, [0.06, 0.30, 0.85, 0.05, 0.02]),
        (63, [0.08, 0.40, 0.65, 0.10, 0.048]),
        (64, [0.09, 0.50, 0.45, 0.15, 0.058]),
        (65, [0.10, 0.60, 0.25, 0.20, 0.068]),
    ] {
        let prepared = prepare(&against(&rogue, level));
        let melee = &prepared["melee"];
        let got = [
            f(&melee["base_miss_chance"]),
            f(&melee["base_glance_chance"]),
            f(&melee["glance_multiplier"]),
            f(&melee["glance_spread"]),
            f(&melee["melee_crit_suppression"]),
        ];
        for (got, want) in got.iter().zip(want) {
            assert!(near(*got, want), "level {level}: {got} for {want}");
        }
        // The boards' encounter takes no hit suppression.
        assert_eq!(f(&melee["hit_suppression"]), 0.0, "level {level}");
    }
    // With the encounter's hit suppression on: Blizzard's 1% from 3 above, none below.
    let mut suppressing = rogue.clone();
    suppressing["encounter"]
        .as_object_mut()
        .unwrap()
        .remove("noHitSuppression");
    for (level, want) in [
        (55, 0.0),
        (60, 0.0),
        (62, 0.0),
        (63, 0.01),
        (64, 0.01),
        (65, 0.01),
    ] {
        let prepared = prepare(&against(&suppressing, level));
        assert_eq!(
            f(&prepared["melee"]["hit_suppression"]),
            want,
            "level {level}"
        );
    }
}

#[test]
fn the_spell_table_follows_the_gap() {
    let mage = request("level-gap/frost-mage");
    for (level, miss) in [
        (10, 0.0),
        (56, 0.0),
        (57, 0.01),
        (58, 0.02),
        (59, 0.03),
        (60, 0.04),
        (61, 0.05),
        (62, 0.06),
        (63, 0.17),
        (64, 0.28),
        (65, 0.39),
    ] {
        let prepared = prepare(&against(&mage, level));
        let got = f(&prepared["player"]["attack_table"]["base_spell_miss_chance"]);
        assert!(near(got, miss), "level {level}: {got} for {miss}");
    }
}

/// An enemy's crit and crushing blow against a tank follow its level, and it misses the tank
/// less the higher it is.
#[test]
fn an_enemy_swings_by_its_level() {
    let tank = request("level-gap/protection-warrior");
    let mut misses = Vec::new();
    // level, the enemy's crit percent, its crushing blow chance.
    for (level, crit, crush) in [
        (50, 3.0, 0.0),
        (59, 4.8, 0.0),
        (60, 5.0, 0.0),
        (62, 5.4, 0.0),
        (63, 5.6, 0.15),
        (64, 5.8, 0.25),
        (65, 6.0, 0.35),
    ] {
        let prepared = prepare(&against(&tank, level));
        passes(&prepared);
        let got = f(&prepared["target"]["stats"]["PhysicalCritPercent"]);
        assert!(near(got, crit), "level {level}: crit {got}");
        let rolls = &prepared["enemy"]["rolls"][0];
        let got = f(&rolls["crush_chance"]);
        assert!(near(got, crush), "level {level}: crush {got}");
        misses.push(f(&rolls["miss_chance"]));
    }
    assert!(
        misses.windows(2).all(|pair| pair[0] > pair[1]),
        "{misses:?}"
    );
    // 0.2% a level: 13 levels lower, 2.6% more.
    assert!(near(misses[0] - misses[4], 0.026), "{misses:?}");
}

/// The fight rolls what the table says: no swing glances off a lower target, half of them off
/// one 4 above and three in five off one 5 above, and the damage falls as the boss rises.
#[test]
fn the_fight_follows_the_table() {
    let rogue = request("level-gap/combat-rogue");
    let mut damage = Vec::new();
    for (level, share) in [
        (55, 0.0),
        (59, 0.0),
        (60, 0.10),
        (63, 0.40),
        (64, 0.50),
        (65, 0.60),
    ] {
        let player = simulate(&prepare(&against(&rogue, level)));
        let (swings, glances) = white_swings(&player);
        assert!(swings > 20_000.0, "level {level}: {swings} swings");
        let got = glances / swings;
        assert!(
            (got - share).abs() < 0.01,
            "level {level}: {got} of the swings glance, for {share}"
        );
        damage.push(dps(&player));
    }
    assert!(
        damage.windows(2).all(|pair| pair[0] > pair[1]),
        "{damage:?}"
    );
    // From 10 below every table is at its floor: a lower boss changes nothing more.
    let floor = dps(&simulate(&prepare(&against(&rogue, 50))));
    assert_eq!(dps(&simulate(&prepare(&against(&rogue, 20)))), floor);
}

#[test]
fn a_levelling_character_fights_level_10_to_5_above() {
    let rogue = request("levels/combat-rogue-L30");
    // level, miss, glance chance.
    for (level, miss, glance) in [
        (10, 0.0, 0.0),
        (25, 0.025, 0.0),
        (29, 0.045, 0.0),
        (30, 0.05, 0.10),
        (32, 0.06, 0.30),
        (33, 0.08, 0.40),
        (34, 0.09, 0.50),
        (35, 0.10, 0.60),
    ] {
        let prepared = prepare(&against(&rogue, level));
        let melee = &prepared["melee"];
        assert!(near(f(&melee["base_miss_chance"]), miss), "level {level}");
        assert!(
            near(f(&melee["base_glance_chance"]), glance),
            "level {level}"
        );
        assert!(dps(&simulate(&prepared)) > 0.0, "level {level}");
    }
    let prepared = prepare(&against(&rogue, 36));
    assert_eq!(
        refusals(&prepared),
        [(
            "level_unsupported".to_string(),
            "target level 36 is not supported".to_string()
        )]
    );
}
