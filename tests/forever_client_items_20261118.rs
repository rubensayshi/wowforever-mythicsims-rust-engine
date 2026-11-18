//! Forever items and set bonuses that refused real players' Quick Sims on 17 and 18 November
//! 2026:
//!
//! - Band of the Damned (276073), "no item with id 276073" twice (Arcane Mage): a ring of plain
//!   stats from Wowhead's Forever tooltip of 18 November 2026 (+11 Intellect, +6 Spirit, up to
//!   15 spell damage and healing), checked against the Item row of client 1.60.1.70959.
//! - Spider's Kiss (2), "player aura Spider's Kiss 2P listens to combat events without an
//!   effect" twice: Venomspitter (13183) and Fang of the Crystal Spider (13218) worn together.
//!
//! Left out, as their effects state no numbers the engine can use: Weed Whacker (274270, a
//! chance on hit with no stated rate) and Totem of Sustaining (23200, Water Shield charges).

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[path = "support/rogue_113483.rs"]
mod rogue_113483;

const BAND_OF_THE_DAMNED: i64 = 276073;
const VENOMSPITTER: i64 = 13183;
const FANG_OF_THE_CRYSTAL_SPIDER: i64 = 13218;

const FINGER_1: usize = 10;
const MAIN_HAND: usize = 14;
const OFF_HAND: usize = 15;

fn player(request: &mut Value) -> &mut Value {
    &mut request["raid"]["parties"][0]["players"][0]
}

/// A fixture's request without buffs, with these items worn in place of the fixture's.
fn dressed(fixture: &str, items: &[(usize, i64)]) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("fixtures/forever-gaps/{fixture}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(20);
    request["raid"]["buffs"] = json!({});
    request["raid"]["parties"][0]["buffs"] = json!({});
    let player = player(&mut request);
    player["buffs"] = json!({});
    let equipment = player["equipment"]["items"].as_array_mut().unwrap();
    equipment.resize(17, json!({}));
    for &(slot, id) in items {
        equipment[slot] = json!({ "id": id });
    }
    request
}

fn rogue(items: &[(usize, i64)]) -> Value {
    let mut request = dressed("combat-rogue-self-casts-at-player", items);
    rogue_113483::forever_request(&mut request);
    request
}

fn shaman(items: &[(usize, i64)]) -> Value {
    dressed("elemental-shaman-the-elements", items)
}

fn prepare(request: &Value) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever)
        .unwrap_or_else(|err| panic!("{err}"))
}

fn run(request: &Value) -> Value {
    let value = prepare(request);
    let prepared: PreparedV2 = serde_json::from_value(value.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|err| panic!("{err:?}"));
    let result = simulate_prepared(&prepared).unwrap().result;
    let player = result["raidMetrics"]["parties"][0]["players"][0].clone();
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0);
    value
}

fn stat(prepared: &Value, name: &str) -> f64 {
    prepared["player"]["stats"][name].as_f64().unwrap_or(0.0)
}

#[test]
fn band_of_the_damned_prepares_with_its_tooltip_stats() {
    let without = prepare(&shaman(&[(FINGER_1, 0)]));
    let with = run(&shaman(&[(FINGER_1, BAND_OF_THE_DAMNED)]));
    // Intellect and Spirit pass through the character's percent bonuses; the flat stats do not.
    assert!((11.0..13.0).contains(&(stat(&with, "Intellect") - stat(&without, "Intellect"))));
    assert!((6.0..8.0).contains(&(stat(&with, "Spirit") - stat(&without, "Spirit"))));
    assert_eq!(
        stat(&with, "SpellDamage") - stat(&without, "SpellDamage"),
        15.0
    );
    assert_eq!(
        stat(&with, "HealingPower") - stat(&without, "HealingPower"),
        15.0
    );
}

#[test]
fn spiders_kiss_pair_passes_the_gate_and_runs() {
    let prepared = run(&rogue(&[
        (MAIN_HAND, VENOMSPITTER),
        (OFF_HAND, FANG_OF_THE_CRYSTAL_SPIDER),
    ]));
    let auras = prepared["player"]["auras"].as_array().unwrap();
    assert!(auras.iter().any(|aura| aura["label"] == "Spider's Kiss 2P"));
    let effects = prepared["effects"].as_array().unwrap();
    assert!(effects.iter().any(
        |effect| effect["kind"] == "stat_proc" && effect["trigger_aura"] == "Spider's Kiss 2P"
    ));
}
