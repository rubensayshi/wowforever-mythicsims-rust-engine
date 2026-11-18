//! A levelling character's spellbook is the word on the spells the class trainer teaches
//! (docs/levels.md, 18 November 2026).
//!
//! The engine read the spellbook only for ladders of several ranks. A spell of one rank, or a
//! rank the store has no row for, was known by its level row, and some of those rows are wrong
//! for Forever: Shadowfiend (401977) reads level 1, Shank's rank 2 (1329549) has no row. Of 476
//! MythicSim levelling sims run on 17 and 18 November, 46 Rogues below level 44 cast Shank (12 of
//! them below 24, where the trainer teaches no rank), 6 Priests cast Shadowfiend, Paladins cast
//! Holy Shock ranks they had not trained, and characters cast Evocation, Berserker Stance or Rapid
//! Fire before buying them. Now a rank of a spell the trainer teaches that the spellbook does not
//! list is unlearned (`data/forever-trainer-spells.jsonl`, `tools/forever_trainer_spells.py`).
//!
//! The fixtures are those players' requests, names removed.

use forever_engine::contracts::prepared_v2::PreparedV2;
use forever_engine::{prepare_json_with, prepared_refusals, simulate_prepared, Rules};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::{fs, path::Path};

const SHANK_RANK_1: i64 = 1_329_283;
const SHANK_RANK_2: i64 = 1_329_549;
const SHADOWFIEND: i64 = 401_977;
const BERSERKER_STANCE: i64 = 2458;
const EVOCATION: i64 = 12051;
const HOLY_SHOCK_RANK_2: i64 = 20473;
const HOLY_SHOCK_RANK_3: i64 = 20929;
const HOLY_SHOCK_RANK_4: i64 = 20930;
const HOLY_SHOCK_HEAL_RANK_2: i64 = 25914;
const HOLY_SHOCK_HEAL_RANK_3: i64 = 25913;
const HOLY_SHOCK_HEAL_RANK_4: i64 = 25903;

const FIXTURES: [&str; 6] = [
    "rogue-L22-no-shank",
    "combat-rogue-L38-shank-rank-1",
    "healer-priest-L38-no-shadowfiend",
    "holy-paladin-L44-holy-shock",
    "warrior-L30-no-berserker-stance",
    "frost-mage-L20-no-evocation",
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn request(name: &str) -> Value {
    let path = root().join(format!("fixtures/levels/{name}.request.json"));
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn book(request: &Value) -> HashSet<i64> {
    request["raid"]["parties"][0]["players"][0]["knownSpells"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_i64)
        .collect()
}

fn trainer_spells() -> HashSet<i64> {
    fs::read_to_string(root().join("data/forever-trainer-spells.jsonl"))
        .unwrap()
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).unwrap()["id"]
                .as_i64()
                .unwrap()
        })
        .collect()
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

/// Casts per spell id of the player's own actions (tag 0).
fn casts(result: &Value) -> HashMap<i64, f64> {
    let mut casts = HashMap::new();
    let player = &result["raidMetrics"]["parties"][0]["players"][0];
    for action in player["actions"].as_array().into_iter().flatten() {
        let Some(id) = action["id"]["spellId"].as_i64() else {
            continue;
        };
        if action["id"]["tag"].as_i64().unwrap_or(0) != 0 {
            continue;
        }
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

/// The player's spells by id, with whether the rotation can press each.
fn pressable(prepared: &Value) -> HashMap<i64, bool> {
    prepared["player"]["spells"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|spell| {
            let id = spell["action_id"]["spell_id"].as_i64()?;
            let flags = spell["flags"].as_array().unwrap();
            let apl = flags.iter().any(|flag| flag == "SpellFlagAPL");
            let unlearned = flags.iter().any(|flag| flag == "SpellFlagUnlearned");
            Some((id, apl && !unlearned))
        })
        .collect()
}

fn run(name: &str) -> (Value, Value, HashMap<i64, f64>) {
    let request = request(name);
    let prepared = prepare(&request);
    let result = simulate(&prepared);
    let casts = casts(&result);
    (request, prepared, casts)
}

#[test]
fn no_fixture_casts_a_trainer_spell_its_spellbook_leaves_out() {
    let trainer = trainer_spells();
    for name in FIXTURES {
        let (request, _, casts) = run(name);
        let book = book(&request);
        assert!(casts.values().sum::<f64>() > 0.0, "{name} casts nothing");
        for (id, count) in &casts {
            // Shank is registered as rank 2 and pressed through rank 1's alias.
            let through_alias = *id == SHANK_RANK_2 && book.contains(&SHANK_RANK_1);
            if *count > 0.0 && trainer.contains(id) && !book.contains(id) && !through_alias {
                panic!("{name} casts {id}, which its spellbook does not list");
            }
        }
    }
}

#[test]
fn shank_waits_for_the_trainer() {
    let (_, prepared, casts) = run("rogue-L22-no-shank");
    assert_eq!(pressable(&prepared).get(&SHANK_RANK_2), Some(&false));
    assert_eq!(casts.get(&SHANK_RANK_2).copied().unwrap_or(0.0), 0.0);

    let (request, prepared, casts) = run("combat-rogue-L38-shank-rank-1");
    assert!(book(&request).contains(&SHANK_RANK_1));
    assert_eq!(pressable(&prepared).get(&SHANK_RANK_2), Some(&true));
    assert!(casts.get(&SHANK_RANK_2).copied().unwrap_or(0.0) > 0.0);
}

/// Shank rank 1 (level 24) generates 15 Energy, rank 2 (level 44) 25; the engine registers rank 2
/// and presses it through rank 1's alias, so the effect carries the known rank's Energy.
#[test]
fn shank_rank_1_generates_rank_1_energy() {
    let shank_gain = |request: &Value| {
        let prepared = prepare(request);
        prepared["effects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|effect| effect["kind"] == "shank")
            .and_then(|effect| effect["energy_gain"].as_f64())
    };
    // This rogue's Flash of Steel adds the same Energy to either rank.
    let mut request = request("combat-rogue-L38-shank-rank-1");
    let rank_1 = shank_gain(&request).unwrap();
    let player = &mut request["raid"]["parties"][0]["players"][0];
    player["level"] = Value::from(44);
    player["knownSpells"]
        .as_array_mut()
        .unwrap()
        .push(Value::from(SHANK_RANK_2));
    let rank_2 = shank_gain(&request).unwrap();
    assert_eq!((rank_1, rank_2 - rank_1), (21.0, 10.0));
}

#[test]
fn shadowfiend_waits_for_the_spellbook() {
    let (_, prepared, casts) = run("healer-priest-L38-no-shadowfiend");
    assert_eq!(pressable(&prepared).get(&SHADOWFIEND), Some(&false));
    assert_eq!(casts.get(&SHADOWFIEND).copied().unwrap_or(0.0), 0.0);
}

/// A level 44 Holy Paladin with the Holy Shock talent (rank 2, 20473) has not trained rank 3
/// (level 48) or 4 (level 56): neither rank nor its heal is pressed, and the rotation's rank 4
/// heal casts rank 2's.
#[test]
fn holy_shock_casts_the_trained_rank() {
    let (request, prepared, casts) = run("holy-paladin-L44-holy-shock");
    let book = book(&request);
    assert!(book.contains(&HOLY_SHOCK_RANK_2));
    assert!(!book.contains(&HOLY_SHOCK_RANK_3) && !book.contains(&HOLY_SHOCK_RANK_4));
    let pressable = pressable(&prepared);
    for id in [
        HOLY_SHOCK_RANK_3,
        HOLY_SHOCK_RANK_4,
        HOLY_SHOCK_HEAL_RANK_3,
        HOLY_SHOCK_HEAL_RANK_4,
    ] {
        assert_eq!(pressable.get(&id), Some(&false), "{id} is pressable");
        assert_eq!(casts.get(&id).copied().unwrap_or(0.0), 0.0, "{id} is cast");
    }
    assert!(casts.get(&HOLY_SHOCK_HEAL_RANK_2).copied().unwrap_or(0.0) > 0.0);
}

#[test]
fn stances_and_cooldowns_wait_for_the_trainer() {
    let (request, _, casts) = run("warrior-L30-no-berserker-stance");
    assert!(!book(&request).contains(&BERSERKER_STANCE));
    assert_eq!(casts.get(&BERSERKER_STANCE).copied().unwrap_or(0.0), 0.0);

    let (request, _, casts) = run("frost-mage-L20-no-evocation");
    assert!(!book(&request).contains(&EVOCATION));
    assert_eq!(casts.get(&EVOCATION).copied().unwrap_or(0.0), 0.0);
}
