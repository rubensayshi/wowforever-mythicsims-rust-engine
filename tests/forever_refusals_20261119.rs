//! Forever gear that refused real players' sims on 18 and 19 November 2026:
//!
//! - Naturopathy Footpads (287038), "no item with id 287038" three times (two Shadow Priest and
//!   one Holy Priest Quick Sim): cloth boots of plain stats from Wowhead's Forever tooltip of 19
//!   November 2026 (58 Armor, +17 Spirit, healing up to 22 and spell damage up to 7), checked
//!   against the Item row of client 1.60.1.70959. Its 18 health per 5 sec has no stat, as on the
//!   reference rows.
//! - Dreadmist Raiment (5), "player aura Dreadmist Raiment 5P listens to combat events without an
//!   effect" (a Shadow Priest's Best Gear run whose bag pieces reach five): "Your spellcasts have
//!   a 5% chance to heal you for 270 to 330." ItemSetSpell 7909 gives ItemSet 183 the spell
//!   450585 at five pieces; its SpellAuraOptions row 236875 states ProcChance 5 on the casts of
//!   magic spells (ProcTypeMask 0x14000), and it casts Dark Reward 450583, whose SpellEffect row
//!   1150582 is a heal of 300 with variance 0.2 and no coefficient (wago.tools `product=wowf`,
//!   build 1.60.1.70959).

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

const NATUROPATHY_FOOTPADS: i64 = 287038;
const DARK_REWARD: i64 = 450583;
const DREADMIST_5: &str = "Dreadmist Raiment 5P";

// Dreadmist Mask, Mantle, Robe, Wraps, Belt, Leggings and Sandals, by slot.
const HEAD: (usize, i64) = (0, 16698);
const SHOULDER: (usize, i64) = (2, 16701);
const CHEST: (usize, i64) = (4, 16700);
const HANDS: (usize, i64) = (6, 16705);
const WAIST: (usize, i64) = (7, 16702);
const LEGS: (usize, i64) = (8, 16699);
const FEET: usize = 9;

const PRIEST: &str = "shadow-priest-vampiric-embrace";

/// The Shadow Priest fixture's request with these items worn in place of the fixture's.
fn dressed(items: &[(usize, i64)], iterations: u32) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("fixtures/forever-gaps/{PRIEST}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(iterations);
    let player = &mut request["raid"]["parties"][0]["players"][0];
    let equipment = player["equipment"]["items"].as_array_mut().unwrap();
    for &(slot, id) in items {
        equipment[slot] = json!({ "id": id });
    }
    request
}

fn prepare(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err}"))
}

/// A prepared input through the gate and run: the player's metrics.
fn simulate(value: &Value) -> Value {
    let prepared: PreparedV2 = serde_json::from_value(value.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|err| panic!("{err:?}"));
    let result = simulate_prepared(&prepared).unwrap().result;
    let player = result["raidMetrics"]["parties"][0]["players"][0].clone();
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0);
    player
}

fn stat(prepared: &Value, name: &str) -> f64 {
    prepared["player"]["stats"][name].as_f64().unwrap_or(0.0)
}

fn dark_reward_effect(prepared: &Value) -> Option<&Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| {
            effect["kind"] == "spell_data_heal_proc" && effect["trigger_aura"] == DREADMIST_5
        })
}

/// A field of the player's actions with this spell id, summed over every tag and target.
fn total(player: &Value, id: i64, field: &str) -> f64 {
    player["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| action["id"]["spellId"].as_i64() == Some(id))
        .flat_map(|action| action["targets"].as_array().unwrap())
        .map(|target| target[field].as_f64().unwrap_or(0.0))
        .sum()
}

#[test]
fn naturopathy_footpads_prepare_with_their_tooltip_stats() {
    let without = prepare(&dressed(&[(FEET, 0)], 20), Rules::Forever);
    let with = prepare(
        &dressed(&[(FEET, NATUROPATHY_FOOTPADS)], 20),
        Rules::Forever,
    );
    simulate(&with);
    // Spirit passes through the character's percent bonuses (a Human Priest's talents make 21
    // of it); the flat stats do not.
    assert!((17.0..=22.0).contains(&(stat(&with, "Spirit") - stat(&without, "Spirit"))));
    assert_eq!(stat(&with, "Armor") - stat(&without, "Armor"), 58.0);
    assert_eq!(
        stat(&with, "HealingPower") - stat(&without, "HealingPower"),
        22.0
    );
    assert_eq!(
        stat(&with, "SpellDamage") - stat(&without, "SpellDamage"),
        7.0
    );
}

#[test]
fn four_dreadmist_pieces_register_no_heal() {
    let prepared = prepare(
        &dressed(&[HEAD, SHOULDER, CHEST, HANDS], 20),
        Rules::Forever,
    );
    assert!(dark_reward_effect(&prepared).is_none());
    let player = simulate(&prepared);
    assert_eq!(total(&player, DARK_REWARD, "healing"), 0.0);
}

/// Five pieces pass the gate, and the heal is the rows': each one 270 to 330 before a crit, with
/// no spell power in it.
#[test]
fn five_dreadmist_pieces_pass_the_gate_and_heal_as_the_rows_state() {
    let prepared = prepare(
        &dressed(&[HEAD, SHOULDER, CHEST, HANDS, WAIST, LEGS], 400),
        Rules::Forever,
    );
    let auras = prepared["player"]["auras"].as_array().unwrap();
    assert!(auras.iter().any(|aura| aura["label"] == DREADMIST_5));
    let effect = dark_reward_effect(&prepared).expect("the five piece bonus has its effect");
    assert_eq!(effect["proc_chance"], 0.05);
    assert_eq!(effect["average"], 300.0);
    assert!((effect["variance"].as_f64().unwrap() - 0.2).abs() < 1e-6);
    assert_eq!(effect["casts"], true);
    // The heal is no cast that rolls its own proc.
    let heal = effect["spell"].as_u64().unwrap();
    let heard = effect["trigger_spells"].as_array().unwrap();
    assert!(!heard.is_empty());
    assert!(heard.iter().all(|spell| spell.as_u64() != Some(heal)));

    let player = simulate(&prepared);
    let hits = total(&player, DARK_REWARD, "hits");
    let crits = total(&player, DARK_REWARD, "crits");
    let healing = total(&player, DARK_REWARD, "healing");
    assert!(hits + crits > 0.0, "Dark Reward never healed");
    // Every heal is 270 to 330, half again on a crit.
    assert!(
        healing >= 270.0 * (hits + crits) && healing <= 330.0 * (hits + 1.5 * crits),
        "{healing} healing over {hits} hits and {crits} crits"
    );
}

/// The reference's rules register the listener alone, as before.
#[test]
fn the_reference_rules_keep_the_listener_without_a_heal_spell() {
    let prepared = prepare(
        &dressed(&[HEAD, SHOULDER, CHEST, HANDS, WAIST, LEGS], 20),
        Rules::GoReference,
    );
    assert!(dark_reward_effect(&prepared).is_none());
    let spells = prepared["player"]["spells"].as_array().unwrap();
    assert!(spells
        .iter()
        .all(|spell| spell["action_id"]["spell_id"].as_i64() != Some(DARK_REWARD)));
}
