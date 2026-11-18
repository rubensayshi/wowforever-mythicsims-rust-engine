//! A Shaman the boss swings at, against several copies of the boss, under Forever's rules.
//! Every copy swings at a tank on its own timer, and the gate refused a tank whose class had not
//! been checked against that ("4 targets: every copy of the boss swings at the tank, which is
//! unsupported for ClassShaman yet", production sim 32ffb765, 17 November 2026).
//!
//! What a Shaman does on a hit it takes answers the copy that swung or no target at all:
//! Lightning Shield's orb strikes the attacker of the hit, a Water Shield globe gives mana and
//! Improved Stormstrike's reset finishes a cooldown. Under the reference's rules a Shaman tank at
//! 1 to 5 targets matches the pinned Go engine on the whole result and the first fight's log
//! (validation/2026-11-19-shaman-tanking-several-targets.json).
//!
//! The request is the production one, an Enhancement Shaman with Rockbiter Weapon and Lightning
//! Shield in its own rotation at 4 targets (fixtures/forever-gaps, name replaced, 50 iterations).

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::Value;
use std::{fs, path::Path};

fn request() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/forever-gaps/enhancement-shaman-tank-four-targets.request.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

/// The request against `count` copies of its boss.
fn against(count: usize) -> Value {
    let mut request = request();
    let targets = request["encounter"]["targets"].as_array_mut().unwrap();
    targets.truncate(count);
    while targets.len() < count {
        targets.push(targets[0].clone());
    }
    request
}

fn prepared(request: &Value) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever).unwrap()
}

/// The result of a run through the gate.
fn run(prepared: &Value) -> Value {
    let prepared: PreparedV2 = serde_json::from_value(prepared.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|error| panic!("{error:?}"));
    simulate_prepared(&prepared).unwrap().result
}

fn player(result: &Value) -> &Value {
    &result["raidMetrics"]["parties"][0]["players"][0]
}

/// The damage of the player's spell on each target it hit.
fn damage_by_target(player: &Value, spell_id: i64) -> Vec<f64> {
    let mut damage = Vec::new();
    for action in player["actions"].as_array().unwrap() {
        if action["id"]["spellId"].as_i64() != Some(spell_id) {
            continue;
        }
        for (index, target) in action["targets"].as_array().unwrap().iter().enumerate() {
            if damage.len() <= index {
                damage.resize(index + 1, 0.0);
            }
            damage[index] += target["damage"].as_f64().unwrap_or(0.0);
        }
    }
    damage
}

/// A request of the comparison against the reference (validation/2026-11-19-shaman-tanking-
/// several-targets) at 50 iterations, under the reference's rules: the player's DPS and threat.
fn reference_run(name: &str) -> (f64, f64) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "validation/2026-11-19-shaman-tanking-several-targets/{name}.request.json"
    ));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = Value::from(50);
    let prepared =
        prepare_json_with(request.to_string().as_bytes(), "rust", Rules::GoReference).unwrap();
    let result = run(&prepared);
    let player = player(&result);
    (
        player["dps"]["avg"].as_f64().unwrap(),
        player["threat"]["avg"].as_f64().unwrap(),
    )
}

/// Improved Stormstrike's reset finishes Stormstrike's cooldown a spell batch window after the
/// dodge or parry, as the reference's proc trigger handlers run, so the rotation casts it at its
/// next look and not at the moment of the swing. The production Enhancement Shaman tanking one
/// target matches the pinned Go engine to the last digit (Go a667dce84, 50 iterations, seed 42);
/// with the reset at once it read 628.2467013509752 DPS.
#[test]
fn improved_stormstrikes_reset_waits_a_batch_window_as_the_reference() {
    assert_eq!(
        reference_run("tank-production-enhancement-shaman"),
        (630.1411053692136, 801.2939713414228)
    );
}

/// The same Shaman against four copies of the boss matches the reference likewise.
#[test]
fn a_shaman_tank_matches_the_reference_at_four_targets() {
    assert_eq!(
        reference_run("tank-production-enhancement-shaman-4-targets"),
        (873.6317358729298, 1172.8378981902563)
    );
}

/// The production request prepares, passes the gate and runs, and each of the four copies
/// swings at the Shaman.
#[test]
fn a_shaman_tanks_four_copies_of_the_boss() {
    let value = prepared(&request());
    assert_eq!(value["encounter"]["target_count"], 4);
    assert!(!value["enemy"].is_null(), "the boss swings at the Shaman");
    let result = run(&value);
    assert!(player(&result)["dps"]["avg"].as_f64().unwrap() > 0.0);
    let targets = result["encounterMetrics"]["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 4);
    for (index, target) in targets.iter().enumerate() {
        let dps = target["dps"]["avg"].as_f64().unwrap_or(0.0);
        assert!(dps > 0.0, "target {} deals {dps}", index + 1);
    }
}

/// Lightning Shield's orb strikes the copy whose swing spent the charge: over the fights each
/// of the four copies is struck.
#[test]
fn lightning_shields_orb_strikes_the_copy_that_swung() {
    let value = prepared(&request());
    let shield = value["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "lightning_shield")
        .expect("Lightning Shield");
    let orb = shield["orb"]["spell_id"].as_i64().expect("the orb's spell");
    let result = run(&value);
    // An action lists every unit it can land on: the four copies, then the Shaman.
    let damage = damage_by_target(player(&result), orb);
    assert_eq!(damage.len(), 5, "{damage:?}");
    assert!(damage[..4].iter().all(|damage| *damage > 0.0), "{damage:?}");
    assert_eq!(damage[4], 0.0, "{damage:?}");
}

/// Four copies swinging deal about four times the damage of one: each swings on its own timer.
#[test]
fn four_copies_hit_about_four_times_as_often_as_one() {
    let taken = |count: usize| {
        let result = run(&prepared(&against(count)));
        let targets = result["encounterMetrics"]["targets"].as_array().unwrap();
        assert_eq!(targets.len(), count);
        targets
            .iter()
            .map(|target| target["dps"]["avg"].as_f64().unwrap())
            .sum::<f64>()
    };
    let (one, four) = (taken(1), taken(4));
    let ratio = four / one;
    assert!(
        (3.5..4.5).contains(&ratio),
        "{one} from one, {four} from four"
    );
}
