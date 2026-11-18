//! Shield Slam off the target's threat, under Forever's rules. The hit adds the warrior's block
//! value, which the runtime read from the rolls of the target's swing at a tank, so a warrior
//! nothing swings at was refused ("rotation reaches spell 23922, which reads the block value
//! without a tanked target", production sim 18058183, 17 November 2026).
//!
//! The Shield Slam effect now states the block value itself for such a warrior, under each stat
//! aura combination, read from the reset simulations the stat auras effect reads. It is the
//! number a tank's rolls carry: the same character tanking has the same value in every
//! combination.
//!
//! The request is the production one, a level 42 Orc Warrior with a shield on the DPS Warrior
//! spec (fixtures/forever-gaps, name replaced, 50 iterations).

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// The rank the level 42 Warrior knows, which the rotation's rank 4 line reaches.
const SHIELD_SLAM: i64 = 23922;

fn request() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/forever-gaps/dps-warrior-shield-slam-off-threat.request.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn prepared(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules).unwrap()
}

fn effect<'a>(prepared: &'a Value, kind: &str) -> &'a Value {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} effect"))
}

fn refusal(prepared: &Value) -> String {
    let prepared: PreparedV2 = serde_json::from_value(prepared.clone()).unwrap();
    match check_prepared(&prepared) {
        Ok(()) => String::new(),
        Err(error) => format!("{error:?}"),
    }
}

/// The player's metrics of a run through the gate.
fn run(prepared: &Value) -> Value {
    let refused = refusal(prepared);
    assert!(refused.is_empty(), "{refused}");
    let prepared: PreparedV2 = serde_json::from_value(prepared.clone()).unwrap();
    let result = simulate_prepared(&prepared).unwrap().result;
    result["raidMetrics"]["parties"][0]["players"][0].clone()
}

/// Casts and damage of the player's Shield Slam.
fn shield_slam(player: &Value) -> (f64, f64) {
    let mut totals = (0.0, 0.0);
    for action in player["actions"].as_array().unwrap() {
        if action["id"]["spellId"].as_i64() != Some(SHIELD_SLAM) {
            continue;
        }
        for target in action["targets"].as_array().unwrap() {
            totals.0 += target["casts"].as_f64().unwrap_or(0.0);
            totals.1 += target["damage"].as_f64().unwrap_or(0.0);
        }
    }
    totals
}

fn block_reductions(prepared: &Value) -> Vec<f64> {
    effect(prepared, "shield_slam")["block_reductions"]
        .as_array()
        .map(|values| values.iter().map(|value| value.as_f64().unwrap()).collect())
        .unwrap_or_default()
}

/// The production request prepares, passes the gate and runs, and Shield Slam hits.
#[test]
fn shield_slam_runs_off_the_targets_threat() {
    let value = prepared(&request(), Rules::Forever);
    assert!(value["enemy"].is_null(), "nothing swings at the warrior");
    let reductions = block_reductions(&value);
    let combinations = effect(&value, "stat_auras")["combos"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(reductions.len(), combinations);
    assert!(
        reductions
            .iter()
            .all(|value| *value > 0.0 && value.fract() == 0.0),
        "{reductions:?}"
    );
    assert!(effect(&value, "shield_slam")["block_value_auras"].is_null());
    let player = run(&value);
    let (casts, damage) = shield_slam(&player);
    assert!(
        casts > 0.0 && damage > 0.0,
        "{casts} casts, {damage} damage"
    );
}

/// The value stated is the one a tank's rolls carry: the same warrior with the target swinging
/// at it has the same stat auras and the same block value in every combination, and states
/// none on the effect.
#[test]
fn the_block_value_is_the_one_a_tanks_rolls_carry() {
    let value = prepared(&request(), Rules::Forever);
    let mut tanking = request();
    tanking["raid"]["tanks"] = json!([{"index": 0, "type": "Player"}]);
    let tanking = prepared(&tanking, Rules::Forever);
    assert_eq!(
        effect(&value, "stat_auras")["auras"],
        effect(&tanking, "stat_auras")["auras"]
    );
    let rolls: Vec<f64> = tanking["enemy"]["rolls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rolls| rolls["block_reduction"].as_f64().unwrap())
        .collect();
    assert_eq!(block_reductions(&value), rolls);
    assert!(block_reductions(&tanking).is_empty());
}

/// The runtime adds the stated value, twice under Forever's rules: 100 more block value in
/// every combination is 200 more damage a landed Shield Slam before armor, crits and the
/// target's modifiers, with every roll of the fight the same.
#[test]
fn shield_slam_adds_the_stated_block_value() {
    let value = prepared(&request(), Rules::Forever);
    let (casts, damage) = shield_slam(&run(&value));
    let mut raised = value.clone();
    let slam = raised["effects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|effect| effect["kind"] == "shield_slam")
        .unwrap();
    assert_eq!(slam["twice_block_value"], json!(true));
    for reduction in slam["block_reductions"].as_array_mut().unwrap() {
        *reduction = json!(reduction.as_f64().unwrap() + 100.0);
    }
    let (raised_casts, raised_damage) = shield_slam(&run(&raised));
    assert_eq!(casts, raised_casts);
    assert!(
        raised_damage > damage + 50.0 * casts,
        "{damage} to {raised_damage} in {casts} casts"
    );
}

/// An aura outside the stat auras that would change the block value is one the runtime could
/// not follow, so the gate refuses a warrior whose effect names one.
#[test]
fn an_aura_that_changes_the_block_value_is_refused() {
    let mut value = prepared(&request(), Rules::Forever);
    let slam = value["effects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|effect| effect["kind"] == "shield_slam")
        .unwrap();
    slam["block_value_auras"] = json!(["Shield of Tests"]);
    let refused = refusal(&value);
    assert!(
        refused.contains("whose block value Shield of Tests would change"),
        "{refused}"
    );
}

/// An effect that states no block value, as the reference's rules prepare it, stays refused:
/// the runtime has nowhere to read one.
#[test]
fn an_effect_without_the_block_value_is_refused() {
    let mut value = prepared(&request(), Rules::Forever);
    let slam = value["effects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|effect| effect["kind"] == "shield_slam")
        .unwrap();
    slam.as_object_mut().unwrap().remove("block_reductions");
    let refused = refusal(&value);
    assert!(
        refused.contains("reads the block value without a tanked target"),
        "{refused}"
    );
}
