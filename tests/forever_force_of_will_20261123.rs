//! Force of Will (11810) under Forever's rules: "Equip: When struck in combat has a chance of
//! decreasing all damage taken by 51 for 10 sec." Its equip spell 15594 triggers 15595 when the
//! wearer is struck by a melee hit that lands: ProcChance 100 (ProcChanceSource 3, which the
//! owner's decision of 13 November 2026 reads as the rate) and a 30 sec ProcCategoryRecovery in
//! the client's SpellAuraOptions (1.60.1). 15595 is a Mod Damage Taken of -51 on every school
//! (Misc 127) for 10 sec.
//!
//! The reference registers 15595 as an equip aura, so its -51 is permanent; that stays the
//! reference's rules. The tests dress the production Protection Warrior of
//! `fixtures/mage/prepared-v2`, who tanks the target, and the DPS Warrior of
//! `fixtures/forever-gaps`, whom no target attacks.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[path = "support/warrior_trees_20261104.rs"]
mod warrior_trees_20261104;

const FORCE_OF_WILL: i64 = 11810;
const FORCE_OF_WILL_BUFF: i64 = 15595;
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

fn dtps(player: &Value) -> f64 {
    player["dtps"]["avg"].as_f64().unwrap()
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
        .find(|aura| aura["id"]["spellId"].as_i64() == Some(FORCE_OF_WILL_BUFF))
}

/// A landed melee hit taken activates the buff, at most once in 30 sec, for 10 sec: it is up
/// about a third of the fight, and cuts the damage taken by much less than the reference's
/// permanent -51.
#[test]
fn force_of_will_is_a_struck_proc_of_ten_seconds_in_thirty() {
    let (prepared, player) = run(&tank_warrior(Some(FORCE_OF_WILL)), Rules::Forever);
    let proc = effect_of(&prepared, "spell_data_stat_proc", "Force of Will Trigger")
        .expect("the struck proc");
    assert_eq!(proc["aura"], "Force of Will");
    assert_eq!(proc["struck"], true);
    assert_eq!(proc["landed_only"], true);
    assert_eq!(proc["proc_chance"], 1.0);
    let trigger = player_aura(&prepared, "Force of Will Trigger").unwrap();
    assert_eq!(
        trigger["icd"]["duration_ns"], 30_000_000_000_i64,
        "{trigger}"
    );
    let buff = player_aura(&prepared, "Force of Will").expect("the buff");
    assert_eq!(buff["duration_ns"], 10_000_000_000_i64, "{buff}");
    // The buff is a stat aura whose combinations carry the swing's rolls.
    let stat_auras = prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "stat_auras")
        .expect("stat auras");
    assert!(stat_auras["auras"]
        .as_array()
        .unwrap()
        .iter()
        .any(|label| label == "Force of Will"));
    let bonus: Vec<f64> = prepared["enemy"]["rolls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rolls| rolls["bonus_damage_taken"].as_f64().unwrap())
        .collect();
    assert!(bonus.contains(&0.0) && bonus.contains(&-51.0), "{bonus:?}");

    // Over 120 sec (15 sec of variation) the buff goes up at most every 30 sec: at most five
    // times, each for at most 10 sec.
    let metrics = buff_metrics(&player).expect("the buff's metrics");
    let procs = metrics["procsAvg"].as_f64().unwrap();
    let uptime = metrics["uptimeSecondsAvg"].as_f64().unwrap();
    assert!((3.0..=5.0).contains(&procs), "{procs}");
    assert!(uptime > 25.0 && uptime <= procs * 10.0 + 1e-9, "{uptime}");
    assert!(uptime < 50.0, "{uptime}");

    // The damage taken: without the trinket, with the proc, and with the reference's permanent
    // -51 on every hit.
    let (_, plain) = run(&tank_warrior(None), Rules::Forever);
    let (reference, permanent) = run(&tank_warrior(Some(FORCE_OF_WILL)), Rules::GoReference);
    assert!(player_aura(&reference, "Force of Will Trigger").is_none());
    let proc_cut = dtps(&plain) - dtps(&player);
    let permanent_cut = dtps(&plain) - dtps(&permanent);
    assert!(permanent_cut > 0.0, "{permanent_cut}");
    // The buff is up about a third of the fight; the runs' random streams part ways, so the
    // bound leaves room for their noise.
    assert!(
        proc_cut > 0.0 && proc_cut < 0.75 * permanent_cut,
        "the proc cuts {proc_cut} a second, the permanent aura {permanent_cut}"
    );
}

/// Worn by a character no target attacks, the listener is inert and the buff is no stat aura.
#[test]
fn force_of_will_is_inert_off_the_tank() {
    let (prepared, player) = run(&dps_warrior(Some(FORCE_OF_WILL)), Rules::Forever);
    assert!(effect_of(&prepared, "inert_listener", "Force of Will Trigger").is_some());
    assert!(effect_of(&prepared, "spell_data_stat_proc", "Force of Will Trigger").is_none());
    assert!(!prepared["effects"]
        .to_string()
        .contains("\"Force of Will\""));
    let uptime = buff_metrics(&player).map_or(0.0, |metrics| {
        metrics["uptimeSecondsAvg"].as_f64().unwrap_or(0.0)
    });
    assert_eq!(uptime, 0.0);
}
