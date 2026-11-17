//! A levelling paladin's Holy Shield is the rank it knows (Forever's rules, 18 November 2026).
//!
//! Push 113554 moved Holy Shield's ranks 1 to 3 to levels 30/40/50 (the frozen rows state
//! 40/50/60) and build 70755 added rank 4 (1239570) at 60, a row the store does not carry. A level
//! 35 Protection Paladin whose rotation casts rank 4 (a MythicSim Quick Sim of a player's
//! export, the name removed) cast rank 4 at 221 a block: every rank was registered at any level,
//! and the rotation rewrite found no level for rank 4. It casts rank 1 now, at 75 damage and 100
//! mana, as the client's SpellLevels rows (wago.tools, product wowf) teach it at 30.

use forever_engine::contracts::prepared_v2::PreparedV2;
use forever_engine::{prepare_json_with, prepared_refusals, simulate_prepared, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

const RANK_1: i64 = 20925;
const RANK_2: i64 = 20927;
const RANK_3: i64 = 20928;
const RANK_4: i64 = 1_239_570;

fn request() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/levels/protection-paladin-L35-holy-shield.request.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn player(request: &mut Value) -> &mut Value {
    &mut request["raid"]["parties"][0]["players"][0]
}

fn prepare(request: &Value) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever)
        .unwrap_or_else(|err| panic!("prepares: {err}"))
}

fn simulate(prepared: &Value) -> Value {
    let typed: PreparedV2 = serde_json::from_value(prepared.clone()).unwrap();
    let refusals = prepared_refusals(&typed);
    assert!(refusals.is_empty(), "gate refusals: {refusals:?}");
    simulate_prepared(&typed).unwrap().result
}

/// The Holy Shield ranks the player has as spells, by spell id.
fn shield_spells(prepared: &Value) -> Vec<i64> {
    prepared["player"]["spells"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|spell| spell["action_id"]["tag"].as_i64().unwrap_or(0) == 0)
        .filter_map(|spell| spell["action_id"]["spell_id"].as_i64())
        .filter(|id| [RANK_1, RANK_2, RANK_3, RANK_4].contains(id))
        .collect()
}

fn spell(prepared: &Value, id: i64) -> &Value {
    prepared["player"]["spells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|spell| {
            spell["action_id"]["spell_id"] == id
                && spell["action_id"]["tag"].as_i64().unwrap_or(0) == 0
        })
        .unwrap_or_else(|| panic!("no spell {id}"))
}

/// The prepared Holy Shield effect's ranks.
fn shield_ranks(prepared: &Value) -> Vec<Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "holy_shield")
        .expect("a holy_shield effect")["ranks"]
        .as_array()
        .unwrap()
        .clone()
}

/// The player's Holy Shield casts, and the hits and damage of its blocks, of one rank.
fn shield_metrics(result: &Value, id: i64) -> (f64, f64, f64) {
    let mut out = (0.0, 0.0, 0.0);
    let unit = &result["raidMetrics"]["parties"][0]["players"][0];
    for action in unit["actions"].as_array().into_iter().flatten() {
        if action["id"]["spellId"].as_i64() != Some(id) {
            continue;
        }
        let proc = action["id"]["tag"].as_i64() == Some(2);
        for target in action["targets"].as_array().into_iter().flatten() {
            if proc {
                out.1 += target["hits"].as_f64().unwrap_or(0.0);
                out.2 += target["damage"].as_f64().unwrap_or(0.0);
            } else {
                out.0 += target["casts"].as_f64().unwrap_or(0.0);
            }
        }
    }
    out
}

/// The reported character: level 35, a spellbook that lists rank 1, a rotation that casts rank 4.
/// It has rank 1 alone, at push 113554's 75 damage and 100 mana, and casts it.
#[test]
fn a_level_35_paladin_casts_holy_shield_rank_1_for_a_rotation_naming_rank_4() {
    let request = request();
    let rotation = request["raid"]["parties"][0]["players"][0]["rotation"].to_string();
    assert!(
        rotation.contains(&RANK_4.to_string()),
        "the rotation names rank 4"
    );
    let prepared = prepare(&request);
    assert_eq!(shield_spells(&prepared), vec![RANK_1]);
    assert_eq!(spell(&prepared, RANK_1)["cost"]["base_cost"], 100);
    let ranks = shield_ranks(&prepared);
    assert_eq!(ranks.len(), 1, "{ranks:?}");
    assert_eq!(ranks[0]["spell_id"], RANK_1);
    assert_eq!(ranks[0]["damage"], 75.0);

    let result = simulate(&prepared);
    let (casts, hits, damage) = shield_metrics(&result, RANK_1);
    assert!(casts > 0.0, "rank 1 is never cast");
    assert!(hits > 0.0, "rank 1 never blocks for damage");
    assert_eq!(damage / hits, 75.0, "every block deals rank 1's 75");
    assert_eq!(shield_metrics(&result, RANK_4), (0.0, 0.0, 0.0));
}

/// Without a spellbook the level decides, at Forever's 30/40/50/60: rank 1 at 35, rank 2 at 45,
/// rank 3 at 55, none at 29 (rank 1 stands in, unlearned, and is never pressed).
#[test]
fn without_a_spellbook_holy_shield_follows_forever_levels() {
    for (level, rank, damage, cost) in [
        (35, RANK_1, 75.0, 100),
        (45, RANK_2, 110.0, 150),
        (55, RANK_3, 153.0, 195),
    ] {
        let mut request = request();
        player(&mut request)
            .as_object_mut()
            .unwrap()
            .remove("knownSpells");
        player(&mut request)["level"] = json!(level);
        for target in request["encounter"]["targets"].as_array_mut().unwrap() {
            target["level"] = json!(level + 2);
        }
        let prepared = prepare(&request);
        assert_eq!(shield_spells(&prepared), vec![rank], "level {level}");
        assert_eq!(spell(&prepared, rank)["cost"]["base_cost"], cost);
        let ranks = shield_ranks(&prepared);
        assert_eq!(ranks.len(), 1, "level {level}: {ranks:?}");
        assert_eq!(ranks[0]["spell_id"], rank);
        assert_eq!(ranks[0]["damage"], damage);
    }

    let mut request = request();
    player(&mut request)
        .as_object_mut()
        .unwrap()
        .remove("knownSpells");
    player(&mut request)["level"] = json!(29);
    for target in request["encounter"]["targets"].as_array_mut().unwrap() {
        target["level"] = json!(31);
    }
    let prepared = prepare(&request);
    let flags = spell(&prepared, RANK_1)["flags"].to_string();
    assert!(flags.contains("SpellFlagUnlearned"), "{flags}");
    assert!(!flags.contains("SpellFlagAPL"), "{flags}");
}
