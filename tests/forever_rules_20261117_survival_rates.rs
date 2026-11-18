//! Forever's rules of 17 November 2026, each from counted combat logs of client 1.60.1.70938:
//!
//! - Predator's Edge reaches Raptor Strike's crit damage bonus twice, compounded.
//! - Measured chance on hit rates: Destiny 1.2, Skullforge Reaver 1.95, Flurry Axe 1.65 and
//!   Heart of Wyrmthalak 2 procs a minute.
//!
//! The counts are in the comments at each rule. The Go reference keeps its own rules.

use forever_engine::{prepare_json_with, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// The live Retribution reference's talents on the launch tree, which only Forever's rules take.
const RETRIBUTION: &str = "050050002-503-05233301001301351";

/// The fixture's main hand weapon's position in its equipment list.
const MAIN_HAND: usize = 12;

/// The fixture's first trinket's position in its equipment list.
const TRINKET: usize = 10;

fn request(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("fixtures/mage/prepared-v2/{name}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(50);
    request["simOptions"]["debugFirstIteration"] = json!(false);
    request
}

fn prepare(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err:?}"))
}

/// The Retribution reference wearing this item in this slot, the rest of its gear kept, on the
/// launch tree under Forever's rules.
fn retribution_wearing(slot: usize, id: i64, rules: Rules) -> Value {
    let mut request = request("production-retribution-paladin");
    let player = &mut request["raid"]["parties"][0]["players"][0];
    if rules == Rules::Forever {
        player["talentsString"] = json!(RETRIBUTION);
    }
    player["equipment"]["items"][slot] = json!({ "id": id });
    request
}

/// The chance a swing rolls the item's chance on hit, from the proc whose trigger aura names it.
fn swing_chance(prepared: &Value, item: &str) -> f64 {
    let proc = prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| {
            effect["trigger_aura"]
                .as_str()
                .is_some_and(|aura| aura.contains(item))
        })
        .unwrap_or_else(|| panic!("{item}'s chance on hit"));
    let chances = proc["chances"].as_array().unwrap();
    assert!(!chances.is_empty(), "{item} rolls on no hit");
    chances[0]["chance"].as_f64().unwrap()
}

fn main_hand_speed(prepared: &Value) -> f64 {
    prepared["melee"]["main_hand"]["swing_speed"]
        .as_f64()
        .unwrap()
}

// ---------------------------------------------------------------------------------------------
// Predator's Edge.
// ---------------------------------------------------------------------------------------------

fn crit_multiplier_additive(prepared: &Value, class_spell: &str, tag: Option<i64>) -> f64 {
    prepared["player"]["spells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|spell| {
            spell["class_spell"] == class_spell && spell["action_id"]["tag"].as_i64() == tag
        })
        .unwrap_or_else(|| panic!("no {class_spell}"))["crit_multiplier_additive"]
        .as_f64()
        .unwrap()
}

/// At Predator's Edge 5/5 Raptor Strike's hit carries the share twice, compounded (0.3 + 0.39,
/// a crit of 2.69 times the hit), while Mongoose Bite keeps the share once (2.30).
#[test]
fn predators_edge_reaches_raptor_strike_twice() {
    let prepared = prepare(&request("production-survival-hunter-melee"), Rules::Forever);
    let raptor = crit_multiplier_additive(&prepared, "raptor_strike", Some(1));
    let bite = crit_multiplier_additive(&prepared, "mongoose_bite", None);
    assert!((bite - 0.3).abs() < 1e-9, "Mongoose Bite: {bite}");
    assert!(
        (raptor - (0.3 + 0.3 * 1.3)).abs() < 1e-9,
        "Raptor Strike: {raptor}"
    );
    let multiplier = 1.0 + (1.0 + raptor);
    assert!((multiplier - 2.69).abs() < 1e-9, "{multiplier}");
}

// ---------------------------------------------------------------------------------------------
// Measured chance on hit rates.
// ---------------------------------------------------------------------------------------------

/// Destiny (647) 1.2, Skullforge Reaver (13361) 1.95 and Flurry Axe (871) 1.65, each by its
/// own speed.
#[test]
fn measured_weapon_rates() {
    for (id, name, ppm) in [
        (647, "Destiny", 1.2),
        (13361, "Skullforge Reaver", 1.95),
        (871, "Flurry Axe", 1.65),
    ] {
        let prepared = prepare(
            &retribution_wearing(MAIN_HAND, id, Rules::Forever),
            Rules::Forever,
        );
        let speed = main_hand_speed(&prepared);
        let chance = swing_chance(&prepared, name);
        assert!(
            (chance - ppm * speed / 60.0).abs() < 1e-9,
            "{name}: {chance} at {speed}"
        );
    }
}

/// The reference keeps Skullforge Reaver's 1.7 and Flurry Axe's 1.9.
#[test]
fn the_reference_keeps_its_weapon_rates() {
    for (id, name, ppm) in [(13361, "Skullforge Reaver", 1.7), (871, "Flurry Axe", 1.9)] {
        let prepared = prepare(
            &retribution_wearing(MAIN_HAND, id, Rules::GoReference),
            Rules::GoReference,
        );
        let speed = main_hand_speed(&prepared);
        let chance = swing_chance(&prepared, name);
        assert!(
            (chance - ppm * speed / 60.0).abs() < 1e-9,
            "{name}: {chance} at {speed}"
        );
    }
}

/// Heart of Wyrmthalak (22321) rolls 2 a minute of the main hand's speed under Forever's rules
/// and 0.4 under the reference's.
#[test]
fn heart_of_wyrmthalak_rate() {
    for (rules, ppm) in [(Rules::Forever, 2.0), (Rules::GoReference, 0.4)] {
        let prepared = prepare(&retribution_wearing(TRINKET, 22321, rules), rules);
        let speed = main_hand_speed(&prepared);
        let chance = swing_chance(&prepared, "Heart of Wyrmthalak");
        assert!(
            (chance - ppm * speed / 60.0).abs() < 1e-9,
            "{rules:?}: {chance} at {speed}"
        );
    }
}
