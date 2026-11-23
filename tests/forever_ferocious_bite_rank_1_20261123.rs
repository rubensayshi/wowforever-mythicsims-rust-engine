//! Ferocious Bite rank 1 (1328618) under Forever's rules: the rank the hotfix of 30 October 2026
//! adds below the frozen ladder, trained at 24 (`data/forever-trainer-spells.jsonl`), a row the
//! store lacks. foreverchanges.pro's druid spellbook of 23 November 2026 states it at 35 Energy,
//! "40-54 damage" at one combo point up to "160-174" at five, and "0.7 additional damage" for
//! each extra Energy. The frozen ladder starts at 22568 (level 32), so a cat of 24 to 31 never
//! bit. The reference's rules keep the frozen ladder.
//!
//! The level 30 cat of `fixtures/levels` (no spellbook), whose rotation names rank 6 (31018).

use forever_engine::contracts::prepared_v2::PreparedV2;
use forever_engine::{prepare_json_with, prepared_refusals, simulate_prepared, Rules};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::{fs, path::Path};

const RANK_1: i64 = 1_328_618;
const RANK_2: i64 = 22_568;

fn request(level: i64) -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/levels/feral-druid-L30.request.json");
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(50);
    request["raid"]["parties"][0]["players"][0]["level"] = json!(level);
    for target in request["encounter"]["targets"].as_array_mut().unwrap() {
        target["level"] = json!(level + 2);
    }
    request
}

fn prepare(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("prepares: {err}"))
}

fn simulate(prepared: &Value) -> Value {
    let typed: PreparedV2 = serde_json::from_value(prepared.clone()).unwrap();
    let refusals = prepared_refusals(&typed);
    assert!(refusals.is_empty(), "gate refusals: {refusals:?}");
    simulate_prepared(&typed).unwrap().result
}

/// The player's spells by spell id, with whether the rotation can press each.
fn spells(prepared: &Value) -> HashMap<i64, bool> {
    prepared["player"]["spells"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|spell| {
            let id = spell["action_id"]["spell_id"].as_i64()?;
            let pressable = spell["flags"]
                .as_array()
                .unwrap()
                .iter()
                .any(|flag| flag == "SpellFlagAPL");
            Some((id, pressable))
        })
        .collect()
}

fn bite_effect(prepared: &Value) -> &Value {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "ferocious_bite")
        .expect("the ferocious_bite effect")
}

/// Casts, landed hits and crits, and damage of the player's action with this spell id.
fn action(result: &Value, spell_id: i64) -> (f64, f64, f64) {
    let player = &result["raidMetrics"]["parties"][0]["players"][0];
    let mut total = (0.0, 0.0, 0.0);
    for action in player["actions"].as_array().into_iter().flatten() {
        if action["id"]["spellId"].as_i64() != Some(spell_id) {
            continue;
        }
        for target in action["targets"].as_array().into_iter().flatten() {
            total.0 += target["casts"].as_f64().unwrap_or(0.0);
            total.1 +=
                target["hits"].as_f64().unwrap_or(0.0) + target["crits"].as_f64().unwrap_or(0.0);
            total.2 += target["damage"].as_f64().unwrap_or(0.0);
        }
    }
    total
}

/// A level 30 cat bites with rank 1: 30 a combo point, 0.7 for each extra Energy, a roll of 17
/// spread over 14, and the 3.7% of attack power a point every Forever rank takes.
#[test]
fn a_level_30_cat_bites_with_rank_1() {
    let prepared = prepare(&request(30), Rules::Forever);
    let registered = spells(&prepared);
    assert_eq!(registered.get(&RANK_1), Some(&true), "{registered:?}");
    assert!(!registered.contains_key(&RANK_2), "{registered:?}");
    let bite = bite_effect(&prepared);
    assert_eq!(bite["damage_per_combo_point"], 30.0);
    assert!(
        (bite["damage_per_energy"].as_f64().unwrap() - 0.7).abs() < 1e-9,
        "{bite}"
    );
    assert_eq!(bite["attack_power_per_combo_point"], 0.037);
    let spell = prepared["player"]["spells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|spell| spell["action_id"]["spell_id"] == RANK_1)
        .unwrap();
    assert_eq!(spell["cost"]["base_cost"], 35, "{spell}");
    let row = &spell["damage_effect"];
    assert_eq!(row["average"], 17.0, "{spell}");
    assert!(
        (row["variance"].as_f64().unwrap() - 14.0 / 17.0).abs() < 1e-9,
        "{spell}"
    );

    // The rotation names rank 6, which the cat presses as rank 1.
    let result = simulate(&prepared);
    let (casts, landed, damage) = action(&result, RANK_1);
    assert!(casts > 0.0, "rank 1 is never pressed");
    // At least 40 a landed bite (one combo point, no attack power), at most the five point roll's
    // 174 with its Energy and attack power doubled for a crit.
    let average = damage / landed;
    assert!((40.0..700.0).contains(&average), "{average} a bite");
}

/// From level 32 the cat bites with 22568, the frozen ladder's first rank, as before.
#[test]
fn from_level_32_the_cat_bites_with_the_frozen_ranks() {
    let prepared = prepare(&request(34), Rules::Forever);
    let registered = spells(&prepared);
    assert_eq!(registered.get(&RANK_2), Some(&true), "{registered:?}");
    assert!(!registered.contains_key(&RANK_1), "{registered:?}");
    assert_eq!(bite_effect(&prepared)["damage_per_combo_point"], 36.0);
}

/// The reference's rules keep the frozen ladder: a level 30 cat has no rank and never bites.
#[test]
fn the_reference_rules_keep_the_frozen_ladder() {
    let prepared = prepare(&request(30), Rules::GoReference);
    let registered = spells(&prepared);
    assert!(!registered.contains_key(&RANK_1), "{registered:?}");
    assert_eq!(registered.get(&RANK_2), Some(&false), "{registered:?}");
}
