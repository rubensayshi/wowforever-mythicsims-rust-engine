//! Mark of the Chosen (17774) under Forever's rules: "Equip: Has a 2% chance when struck in
//! combat of increasing all stats by 21 for 1min." Its equip spell 21969 states ProcChance 2 and
//! a 120 sec proc recovery (ICDMs 120000) and triggers 21970, a Mod Stat of 21 on every stat
//! (Misc -1) for 60 sec. Its proc flags also hold helpful spells taken, which kept the generator
//! from registering it, so the trinket did nothing. The reference's rules still wear it as a
//! plain item.
//!
//! The tests dress the production Protection Warrior of `fixtures/mage/prepared-v2`, who tanks
//! the target, and the DPS Warrior of `fixtures/forever-gaps`, whom no target attacks.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[path = "support/warrior_trees_20261104.rs"]
mod warrior_trees_20261104;

const MARK_OF_THE_CHOSEN: i64 = 17774;
const MARK_OF_THE_CHOSEN_BUFF: i64 = 21970;
const TRINKET: usize = 12;

fn read(path: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn dress(request: &mut Value, trinket: Option<i64>) {
    request["simOptions"]["iterations"] = json!(200);
    request["simOptions"]["debugFirstIteration"] = json!(false);
    let player = &mut request["raid"]["parties"][0]["players"][0];
    let equipment = player["equipment"]["items"].as_array_mut().unwrap();
    equipment.resize(17, json!({}));
    equipment[TRINKET] = match trinket {
        Some(id) => json!({ "id": id }),
        None => json!({}),
    };
}

/// The production Protection Warrior, who tanks the target with a shield.
fn tank_warrior(trinket: Option<i64>) -> Value {
    let mut request = read(
        "fixtures/mage/prepared-v2/protection-warrior-blood-craze-ephemeral-power.request.json",
    );
    warrior_trees_20261104::forever_request(&mut request);
    dress(&mut request, trinket);
    request
}

/// The DPS Warrior of the Pummel request, whom no target attacks.
fn dps_warrior(trinket: Option<i64>) -> Value {
    let mut request = read("fixtures/forever-gaps/dps-warrior-pummel.request.json");
    request["raid"]["buffs"] = json!({});
    request["raid"]["parties"][0]["buffs"] = json!({});
    request["raid"]["parties"][0]["players"][0]["buffs"] = json!({});
    dress(&mut request, trinket);
    request
}

/// Prepared under the rules, through the gate and run: the prepared input and the player's
/// metrics.
fn run(request: &Value, rules: Rules) -> (Value, Value) {
    let value = prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err}"));
    let prepared: PreparedV2 = serde_json::from_value(value.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|err| panic!("{err:?}"));
    let result = simulate_prepared(&prepared).unwrap().result;
    let player = result["raidMetrics"]["parties"][0]["players"][0].clone();
    (value, player)
}

fn effect_of<'a>(prepared: &'a Value, kind: &str, aura: &str) -> Option<&'a Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| {
            effect["kind"] == kind
                && (effect["trigger_aura"] == aura
                    || (kind == "inert_listener" && effect["aura"] == aura))
        })
}

fn player_aura<'a>(prepared: &'a Value, label: &str) -> Option<&'a Value> {
    prepared["player"]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|aura| aura["label"] == label)
}

fn buff_metrics(player: &Value) -> Option<&Value> {
    player["auras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|aura| aura["id"]["spellId"].as_i64() == Some(MARK_OF_THE_CHOSEN_BUFF))
}

/// A landed hit taken activates the buff at a 2% chance, at most once in 120 sec, for 60 sec:
/// in a fight of 120 sec (15 sec of variation) at most twice, and its stats ride the stat aura
/// combinations.
#[test]
fn mark_of_the_chosen_is_a_two_percent_struck_proc() {
    let (prepared, player) = run(&tank_warrior(Some(MARK_OF_THE_CHOSEN)), Rules::Forever);
    let proc = effect_of(
        &prepared,
        "spell_data_stat_proc",
        "Mark of the Chosen Trigger",
    )
    .expect("the struck proc");
    assert_eq!(proc["aura"], "Mark of the Chosen");
    assert_eq!(proc["struck"], true);
    assert_eq!(proc["landed_only"], true);
    assert_eq!(proc["proc_chance"], 0.02);
    let trigger = player_aura(&prepared, "Mark of the Chosen Trigger").unwrap();
    assert_eq!(
        trigger["icd"]["duration_ns"], 120_000_000_000_i64,
        "{trigger}"
    );
    let buff = player_aura(&prepared, "Mark of the Chosen").expect("the buff");
    assert_eq!(buff["duration_ns"], 60_000_000_000_i64, "{buff}");

    // The buff is a stat aura: the combinations with it up carry at least 21 more of each stat
    // (raid buffs may multiply some).
    let stat_auras = prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "stat_auras")
        .expect("stat auras");
    let bit = stat_auras["auras"]
        .as_array()
        .unwrap()
        .iter()
        .position(|label| label == "Mark of the Chosen")
        .expect("a stat aura");
    for stat in ["Strength", "Agility", "Stamina", "Intellect", "Spirit"] {
        assert!(
            stat_auras["changed"]
                .as_array()
                .unwrap()
                .iter()
                .any(|name| name == stat),
            "{stat}"
        );
    }
    let combos = stat_auras["combos"].as_array().unwrap();
    let health = |mask: usize| combos[mask]["Health"].as_f64().unwrap();
    assert!(
        health(1 << bit) - health(0) >= 21.0 * 10.0,
        "{} to {}",
        health(0),
        health(1 << bit)
    );

    let metrics = buff_metrics(&player).expect("the buff's metrics");
    let procs = metrics["procsAvg"].as_f64().unwrap();
    let uptime = metrics["uptimeSecondsAvg"].as_f64().unwrap();
    // A swing lands about every two seconds, so a 2% chance comes in about 100 sec: in most
    // fights once or never, in none more than twice.
    assert!(procs > 0.2 && procs <= 2.0, "{procs}");
    assert!(uptime > 0.0 && uptime <= procs * 60.0 + 1e-9, "{uptime}");

    // The reference's rules wear it as a plain item.
    let (reference, _) = run(&tank_warrior(Some(MARK_OF_THE_CHOSEN)), Rules::GoReference);
    assert!(player_aura(&reference, "Mark of the Chosen Trigger").is_none());
}

/// Worn by a character no target attacks, the listener is inert and the buff is no stat aura.
#[test]
fn mark_of_the_chosen_is_inert_off_the_tank() {
    let (prepared, player) = run(&dps_warrior(Some(MARK_OF_THE_CHOSEN)), Rules::Forever);
    assert!(effect_of(&prepared, "inert_listener", "Mark of the Chosen Trigger").is_some());
    assert!(effect_of(
        &prepared,
        "spell_data_stat_proc",
        "Mark of the Chosen Trigger"
    )
    .is_none());
    assert!(!prepared["effects"]
        .to_string()
        .contains("\"Mark of the Chosen\""));
    let uptime = buff_metrics(&player).map_or(0.0, |metrics| {
        metrics["uptimeSecondsAvg"].as_f64().unwrap_or(0.0)
    });
    assert_eq!(uptime, 0.0);
}
