//! Levelling characters, an audit of every class at every level (docs/levels.md, 18 November
//! 2026).
//!
//! 1,487 MythicSim levelling sims of 12 to 18 November (27 specs, levels 10 to 59) were simmed on
//! bundle-b05aa07 and checked for spells the rotation names and the character knows but never
//! casts, against the same presets at level 60. Three gaps were the engine's:
//!
//! - Lacerate's ladder started at 414644 (level 42), so a bear of 25 to 41 with the talent's own
//!   rank or rank 2 (1328617, 1328616, rows the store lacks) had no Lacerate;
//! - Primal Bite was always rank 3 (level 60), which the spellbook rule leaves out below 60, so a
//!   levelling bear never cast it;
//! - Death Mark's stored damage cap was rank 4's at every level.
//!
//! The fixtures are those players' requests, names removed.

use forever_engine::contracts::prepared_v2::PreparedV2;
use forever_engine::{prepare_json_with, prepared_refusals, simulate_prepared, Rules};
use serde_json::Value;
use std::collections::HashMap;
use std::{fs, path::Path};

const LACERATE_RANK_2: i64 = 1_328_616;
const PRIMAL_BITE_RANK_1: i64 = 407_995;
const PRIMAL_BITE_RANK_3: i64 = 1_238_070;
const BLESSING_OF_WISDOM_RANK_2: i64 = 19850;

fn request(name: &str) -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("fixtures/levels/{name}.request.json"));
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn prepare(request: &Value) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever)
        .unwrap_or_else(|err| panic!("prepares: {err}"))
}

/// Casts per spell id over the run, every tag.
fn run(prepared: &Value) -> HashMap<i64, f64> {
    let typed: PreparedV2 = serde_json::from_value(prepared.clone()).unwrap();
    let refusals = prepared_refusals(&typed);
    assert!(refusals.is_empty(), "gate refusals: {refusals:?}");
    let result = simulate_prepared(&typed).unwrap().result;
    let mut casts = HashMap::new();
    let player = &result["raidMetrics"]["parties"][0]["players"][0];
    for action in player["actions"].as_array().into_iter().flatten() {
        let Some(id) = action["id"]["spellId"].as_i64() else {
            continue;
        };
        let total: f64 = action["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|target| target["casts"].as_f64().unwrap_or(0.0))
            .sum();
        *casts.entry(id).or_default() += total;
    }
    casts
}

fn effect<'a>(prepared: &'a Value, kind: &str) -> &'a Value {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} effect"))
}

/// A level 42 bear with the Lacerate talent and rank 2 (1328616, level 34) and Primal Bite rank 1
/// (407995, level 40) casts both ranks, at their client rows: 6 a stack a tick and 42 flat.
#[test]
fn a_levelling_bear_casts_its_trained_lacerate_and_primal_bite() {
    let prepared = prepare(&request("feral-bear-druid-L42-lacerate-rank-2"));
    assert_eq!(
        effect(&prepared, "lacerate")["tick_base"].as_f64(),
        Some(6.0)
    );
    assert_eq!(
        effect(&prepared, "primal_bite")["flat_damage"].as_f64(),
        Some(42.0)
    );
    let casts = run(&prepared);
    assert!(
        casts.get(&LACERATE_RANK_2).copied().unwrap_or(0.0) > 0.0,
        "{casts:?}"
    );
    assert!(
        casts.get(&PRIMAL_BITE_RANK_1).copied().unwrap_or(0.0) > 0.0,
        "{casts:?}"
    );
    assert_eq!(casts.get(&PRIMAL_BITE_RANK_3), None);

    // Level 36: Lacerate rank 2, and no Primal Bite before 40.
    let prepared = prepare(&request("feral-bear-druid-L36-lacerate-rank-2"));
    let casts = run(&prepared);
    assert!(
        casts.get(&LACERATE_RANK_2).copied().unwrap_or(0.0) > 0.0,
        "{casts:?}"
    );
    assert_eq!(casts.get(&PRIMAL_BITE_RANK_1).copied().unwrap_or(0.0), 0.0);
}

/// A level 49 rogue with Death Mark rank 2 (level 48) stores up to rank 2's cap, 98 and 158 a
/// point (Wowhead Forever's tooltip), not rank 4's 156 and 245.
#[test]
fn death_mark_stores_up_to_the_trained_ranks_cap() {
    let prepared = prepare(&request("subtlety-rogue-L49-death-mark-rank-2"));
    let mark = effect(&prepared, "death_mark");
    assert_eq!(mark["cap_base"].as_f64(), Some(98.0));
    assert_eq!(mark["cap_per_point"].as_f64(), Some(158.0));
}

/// Solo, the worker gives a paladin its own blessing as the level 60 Greater Blessing; a level 26
/// paladin has no Greater Blessing and casts the 5 min Blessing of Wisdom it has trained, rank 2.
#[test]
fn a_levelling_paladin_blesses_itself_with_its_trained_rank() {
    let mut request = request("retribution-paladin-L26-blessing-of-wisdom");
    request["raid"]["parties"][0]["players"][0]["buffs"] =
        serde_json::json!({"greaterBlessingOfWisdom": true});
    let prepared = prepare(&request);
    let wisdom: Vec<i64> = prepared["player"]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|aura| {
            aura["label"]
                .as_str()
                .is_some_and(|label| label.contains("Blessing of Wisdom"))
        })
        .filter_map(|aura| aura["action_id"]["spell_id"].as_i64())
        .collect();
    assert_eq!(wisdom, vec![BLESSING_OF_WISDOM_RANK_2]);
}
