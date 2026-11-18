//! Forever's rules from Blizzard's WoW Forever hotfixes of 16 November 2026 (push 114115), each
//! checked against public Forever combat logs on forever.warcraftlogs.com of 17 November:
//!
//! - Flurry: a Heroic Strike or Cleave spends one charge a cast whatever its outcome and however
//!   many targets it hits (the buff's row now procs on melee abilities). Nine Fury warriors: 94 of
//!   94 landed and 12 of 12 avoided Heroic Strikes and 372 of 372 Cleaves spent a charge with
//!   Flurry up, and 327 of 328 Cleaves on two targets spent one.
//! - Unbridled Wrath and Reckless Swings: Heroic Strike and Cleave activate them, as white swings
//!   do (both rows' ProcFlags went from 0x4 to 0x14).
//! - Windfury Totem: Execute loses its extra attempt, and Shield Slam and Shield Bash no longer
//!   roll it.
//!
//! The Go reference keeps its own rules.

use forever_engine::{prepare_json_with, simulate_prepared, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// The live Fury and Protection references' talents on the launch tree.
const FURY: &str = "30303213002-050530045120100231-2";
const PROTECTION: &str = "-05250035-250033120310021251";
const SHIELD_SLAM: i64 = 23925;
const SHIELD_BASH: i64 = 1672;
const REVENGE: i64 = 11601;

fn request(name: &str, iterations: u32) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("fixtures/mage/prepared-v2/{name}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(iterations);
    request["simOptions"]["debugFirstIteration"] = json!(false);
    request
}

/// A production request with the live reference's talents, which only Forever's rules take,
/// and the party's Windfury Totem up.
fn production(name: &str, talents: &str, rules: Rules, iterations: u32) -> Value {
    let mut request = request(name, iterations);
    request["raid"]["parties"][0]["buffs"]["windfuryTotem"] = json!(true);
    if rules == Rules::Forever {
        request["raid"]["parties"][0]["players"][0]["talentsString"] = json!(talents);
    }
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err:?}"))
}

fn run(prepared: &Value) -> Value {
    let prepared = serde_json::from_value(prepared.clone()).unwrap();
    let report = simulate_prepared(&prepared).unwrap();
    report.result["raidMetrics"]["parties"][0]["players"][0].clone()
}

fn effect<'a>(prepared: &'a Value, kind: &str) -> &'a Value {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} effect"))
}

fn effect_mut<'a>(prepared: &'a mut Value, kind: &str) -> &'a mut Value {
    prepared["effects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|effect| effect["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} effect"))
}

/// Flurry's uptime in seconds an iteration.
fn flurry_uptime(player: &Value) -> f64 {
    let flurry = player["auras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|aura| aura["id"]["spellId"].as_i64() == Some(12966))
        .expect("Flurry");
    flurry["uptimeSecondsAvg"].as_f64().unwrap()
}

/// A Fury warrior that casts Bloodthirst and queues Heroic Strike on every swing.
fn heroic_striker(rules: Rules) -> Value {
    let mut request = request("production-warrior", 300);
    request["raid"]["parties"][0]["buffs"]["windfuryTotem"] = json!(true);
    let player = &mut request["raid"]["parties"][0]["players"][0];
    if rules == Rules::Forever {
        player["talentsString"] = json!(FURY);
    }
    player["rotation"] = json!({"type": "TypeAPL", "prepullActions": [], "priorityList": [
        {"action": {"castSpell": {"spellId": {"spellId": 23894}}}},
        {"action": {"castSpell": {"spellId": {"spellId": 25286}}}},
    ]});
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err:?}"))
}

/// A Heroic Strike or Cleave spends one Flurry charge a cast, so Flurry is up for less of the
/// fight than when only a missed one spent a charge.
#[test]
fn heroic_strike_and_cleave_spend_a_flurry_charge_a_cast() {
    let forever = heroic_striker(Rules::Forever);
    assert_eq!(
        effect(&forever, "warrior_flurry")["strikes_spend_once"],
        json!(true)
    );
    let mut before = forever.clone();
    effect_mut(&mut before, "warrior_flurry")["strikes_spend_once"] = json!(false);
    let (now, then) = (flurry_uptime(&run(&forever)), flurry_uptime(&run(&before)));
    assert!(now < then * 0.95, "Flurry up {now} s against {then} s");

    let reference = production("production-warrior", FURY, Rules::GoReference, 10);
    assert!(effect(&reference, "warrior_flurry")
        .get("strikes_spend_once")
        .is_none());
}

/// Heroic Strike and Cleave activate Unbridled Wrath under Forever's rules only.
#[test]
fn heroic_strike_and_cleave_activate_unbridled_wrath() {
    let forever = production("production-warrior", FURY, Rules::Forever, 10);
    assert_eq!(
        effect(&forever, "unbridled_wrath")["strikes_proc"],
        json!(true)
    );
    let reference = production("production-warrior", FURY, Rules::GoReference, 10);
    assert!(effect(&reference, "unbridled_wrath")
        .get("strikes_proc")
        .is_none());
}

/// Execute rolls the totem once, on its hit: its cast attack no longer does.
#[test]
fn execute_rolls_the_totem_once() {
    for rules in [Rules::Forever, Rules::GoReference] {
        let prepared = production("production-warrior", FURY, rules, 10);
        assert!(effect(&prepared, "execute")
            .get("cast_attack_rolls_windfury_totem")
            .is_none());
    }
}

/// Shield Slam and Shield Bash no longer roll Windfury Totem; Revenge still does.
#[test]
fn shield_strikes_do_not_roll_the_totem() {
    let prepared = production(
        "production-protection-warrior",
        PROTECTION,
        Rules::Forever,
        10,
    );
    let spells = prepared["player"]["spells"].as_array().unwrap();
    let heard: Vec<i64> = effect(&prepared, "windfury_totem")["trigger_spells"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|spell| {
            spells[spell.as_u64().unwrap() as usize]["action_id"]["spell_id"].as_i64()
        })
        .collect();
    assert!(
        !heard.contains(&SHIELD_SLAM) && !heard.contains(&SHIELD_BASH),
        "{heard:?}"
    );
    assert!(heard.contains(&REVENGE), "{heard:?}");
}
