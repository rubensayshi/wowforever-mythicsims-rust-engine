//! Gore Drinker (Fury, 2 ranks), under Forever's rules. There is no Go version: the reference
//! does not simulate it. The numbers are the client's (`classes::warrior::talents::gore_drinker`):
//!
//! - The talent 1323967 (`data/spells.jsonl`, client 1.60.1.70425) and its rank curves
//!   (`data/spell-extras.json`): 3 melee attacks at either rank, 0.5 and 1% of maximum health.
//! - Its buff 1323969 and heal 1323968 on wago.tools `product=wowf` (build 1.60.1.70753, 23
//!   November 2026): SpellAuraOptions 248613, 3 charges at 100% on ProcTypeMask 20 (melee auto
//!   attacks and melee abilities); SpellMisc 869652, SpellDuration 31, 8 sec; SpellEffect 1360683,
//!   a Proc Trigger Spell With Value casting 1323968, whose SpellEffect 1360682 is a Heal.
//!
//! The fight is `fixtures/mage/prepared-v2/fury-warrior-demoralizing-shout`'s Fury Warrior with
//! Gore Drinker added, which casts Bloodrage, Bloodthirst, Death Wish and Berserker Rage.

#[path = "support/warrior_trees_20261104.rs"]
mod warrior_trees_20261104;

use forever_engine::{contracts::prepared_v2::PreparedV2, prepare_json_with, Rules};
use serde_json::Value;
use std::{fs, path::Path};

const HEAL: &str = "{SpellID: 1323968}";
const BUFF: &str = "{SpellID: 1323969}";

/// The fixture's warrior with Gore Drinker at `rank`, its talents in the reference's order.
fn request(rank: u32, iterations: u32) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/mage/prepared-v2/fury-warrior-demoralizing-shout.request.json");
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    // The reference's Fury order holds Gore Drinker at its sixteenth digit.
    request["raid"]["parties"][0]["players"][0]["talentsString"] =
        format!("20315013002-252530051020105{rank}1-2").into();
    request["simOptions"]["iterations"] = iterations.into();
    request
}

fn prepare(request: &Value, rules: Rules) -> Value {
    let mut request = request.clone();
    if rules == Rules::Forever {
        warrior_trees_20261104::forever_request(&mut request);
    }
    prepare_json_with(
        &serde_json::to_vec(&request).unwrap(),
        "gore-drinker",
        rules,
    )
    .unwrap_or_else(|err| panic!("{err}"))
}

fn effect<'a>(prepared: &'a Value, kind: &str) -> Option<&'a Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == kind)
}

fn simulate(prepared: Value) -> Value {
    let prepared: PreparedV2 = serde_json::from_value(prepared).unwrap();
    forever_engine::simulate_prepared(&prepared)
        .unwrap_or_else(|err| panic!("{err:?}"))
        .result
}

/// The client's numbers: 3 charges and 1% at rank 2, 0.5% at rank 1, the buff's 8 sec; the
/// reference prepares nothing.
#[test]
fn the_rows_numbers_are_prepared() {
    for (rank, fraction) in [(1, 0.005), (2, 0.01)] {
        let prepared = prepare(&request(rank, 1), Rules::Forever);
        let gore = effect(&prepared, "gore_drinker").expect("a Gore Drinker effect");
        assert_eq!(gore["charges"], 3);
        assert_eq!(gore["heal_spell_id"], 1323968);
        assert!((gore["health_fraction"].as_f64().unwrap() - fraction).abs() < 1e-12);
        let aura = prepared["player"]["auras"]
            .as_array()
            .unwrap()
            .iter()
            .find(|aura| aura["label"] == "Gore Drinker")
            .expect("the buff");
        assert_eq!(aura["max_stacks"], 3);
        assert_eq!(aura["duration_ns"], 8_000_000_000i64);
    }
    let reference = prepare(&request(2, 1), Rules::GoReference);
    assert!(effect(&reference, "gore_drinker").is_none());
}

/// In the first fight's log each heal restores 1% of the maximum health the line states, under
/// the heal's health metrics, and no more than 3 follow each time the buff comes up.
#[test]
fn each_landed_melee_hit_restores_one_percent_three_times() {
    let result = simulate(prepare(&request(2, 20), Rules::Forever));
    let log = result["logs"].as_str().unwrap();
    let mut heals = 0;
    let mut since_trigger = 0;
    let mut triggers = 0;
    for line in log.lines() {
        if line.contains(&format!("Aura gained: {BUFF}"))
            || line.contains(&format!("Aura refreshed: {BUFF}"))
        {
            triggers += 1;
            since_trigger = 0;
        }
        if let Some(rest) = line.split("Gained ").nth(1) {
            if !rest.contains(&format!("health from {HEAL}")) {
                continue;
            }
            let amount: f64 = rest[..rest.find(' ').unwrap()].parse().unwrap();
            let total: f64 = rest
                .rsplit("of ")
                .next()
                .unwrap()
                .trim_end_matches(" total.")
                .parse()
                .unwrap();
            assert!((amount - total * 0.01).abs() < 0.001, "{line}");
            heals += 1;
            since_trigger += 1;
            assert!(since_trigger <= 3, "a fourth heal: {line}");
        }
    }
    assert!(
        triggers > 0 && heals > 0,
        "{triggers} triggers, {heals} heals"
    );
    let metrics = result["raidMetrics"]["parties"][0]["players"][0]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["id"]["spellId"] == 1323968)
        .expect("the heal's health metrics");
    assert_eq!(metrics["type"], "ResourceTypeHealth");
    assert!(metrics["events"].as_i64().unwrap() > 0);
}

/// Without the talent nothing heals under the heal's metrics.
#[test]
fn without_the_talent_nothing_heals() {
    let prepared = prepare(&request(0, 5), Rules::Forever);
    assert!(effect(&prepared, "gore_drinker").is_none());
    let result = simulate(prepared);
    let resources = result["raidMetrics"]["parties"][0]["players"][0]["resources"]
        .as_array()
        .unwrap();
    assert!(!resources
        .iter()
        .any(|resource| resource["id"]["spellId"] == 1323968));
}
