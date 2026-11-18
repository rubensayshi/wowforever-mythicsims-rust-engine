//! Forever's rules of 16 November 2026, each from counted combat logs or live client rows:
//!
//! - Only a white swing or Windfury Totem's weapon strike refreshes a paladin's judgement
//!   debuffs. Combat logs of a Retribution paladin (client 1.60.1.70938, after the hotfix of 14
//!   November 2026): a refresh followed 396 of 404 lone white swings and 13 of 13 totem strikes,
//!   and none of 173 Holy Strikes, 69 Judgements of Command, 75 Judgements of Righteousness, 23
//!   Judgements of Fury or 2 Hammers of Wrath.
//! - Measured chance on hit rates for seven weapons, from public Forever logs of 14 and 15
//!   November 2026 counted on isolated hits.
//! - Drakefang Butcher's Rend deals twice its damage to Dragonkin (live row and tooltip).
//!
//! The counts are in the comments at each rule. The Go reference keeps its own rules.

use forever_engine::{prepare_json_with, simulate_prepared, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// The live Retribution reference's talents on the launch tree, which only Forever's rules take.
const RETRIBUTION: &str = "050050002-503-05233301001301351";

/// The fixture's main hand weapon's position in its equipment list (Blackblade of Shahram);
/// preparation places an item by its type, so a weapon here replaces it.
const MAIN_HAND: usize = 12;

fn request(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("fixtures/mage/prepared-v2/{name}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(50);
    request["simOptions"]["debugFirstIteration"] = json!(false);
    request
}

fn prepared(request: &Value, rules: Rules) -> Value {
    let mut request = request.clone();
    if rules == Rules::Forever {
        request["raid"]["parties"][0]["players"][0]["talentsString"] = json!(RETRIBUTION);
    }
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err:?}"))
}

fn run(prepared: &Value) -> Value {
    let prepared = serde_json::from_value(prepared.clone()).unwrap();
    simulate_prepared(&prepared).unwrap().result
}

fn effect<'a>(prepared: &'a Value, kind: &str) -> &'a Value {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} effect"))
}

/// The Retribution reference wearing these items in these slots, the rest of its gear kept.
fn retribution_wearing(items: &[(usize, i64)]) -> Value {
    let mut request = request("production-retribution-paladin");
    let equipment = request["raid"]["parties"][0]["players"][0]["equipment"]["items"]
        .as_array_mut()
        .unwrap();
    for (slot, id) in items {
        equipment[*slot] = json!({ "id": id });
    }
    request
}

/// The chance a swing of the weapon rolls its chance on hit, from the proc whose trigger aura
/// names the item.
fn swing_chance(prepared: &Value, item: &str) -> f64 {
    let proc = prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| {
            effect["trigger_aura"]
                .as_str()
                .is_some_and(|aura| aura.contains(item))
        })
        .unwrap_or_else(|| panic!("{item}'s chance on hit"));
    let chances = proc["chances"].as_array().unwrap();
    assert!(!chances.is_empty(), "{item} rolls on no hit");
    let first = chances[0]["chance"].as_f64().unwrap();
    for chance in chances {
        assert!((chance["chance"].as_f64().unwrap() - first).abs() < 1e-12);
    }
    first
}

// ---------------------------------------------------------------------------------------------
// Judgement debuffs.
// ---------------------------------------------------------------------------------------------

/// Under Forever's rules the judgement refresh hears only white swings and the totem's strike;
/// the reference keeps every landed melee hit.
#[test]
fn judgement_refresh_hears_white_swings_only_under_forever_rules() {
    let request = request("production-retribution-paladin");
    let forever = prepared(&request, Rules::Forever);
    assert_eq!(
        effect(&forever, "judgement_refresh")["white_swings_only"],
        true
    );
    let reference = prepared(&request, Rules::GoReference);
    assert!(effect(&reference, "judgement_refresh")
        .get("white_swings_only")
        .is_none());
}

/// White swings still keep Judgement of the Crusader up between judgements: the debuff is on
/// the target for most of the fight under both rules.
#[test]
fn white_swings_keep_the_judgement_up() {
    let request = request("production-retribution-paladin");
    for rules in [Rules::Forever, Rules::GoReference] {
        let result = run(&prepared(&request, rules));
        // Judgement of the Crusader's debuff, every rank.
        let duration = result["encounterMetrics"]["targets"][0]["auras"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|aura| {
                [20188, 20300, 20301, 20302, 20303]
                    .iter()
                    .any(|id| aura["id"]["spellId"] == json!(id))
            })
            .map(|aura| aura["uptimeSecondsAvg"].as_f64().unwrap_or(0.0))
            .fold(0.0, f64::max);
        assert!(duration > 60.0, "{rules:?}: {duration}");
    }
}

// ---------------------------------------------------------------------------------------------
// Measured chance on hit rates.
// ---------------------------------------------------------------------------------------------

/// Stoneslayer (9418) rolls 1.95 a minute and The Black Knight (12974) 0.6, by each weapon's
/// speed, in place of the 1.08 assumed for a rateless chance on hit.
#[test]
fn measured_stoneslayer_and_black_knight_rates() {
    for (id, name, ppm, weapon_speed) in [
        (9418, "Stoneslayer", 1.95, 3.9),
        (12974, "The Black Knight", 0.6, 1.9),
    ] {
        let prepared = prepared(&retribution_wearing(&[(MAIN_HAND, id)]), Rules::Forever);
        let speed = prepared["melee"]["main_hand"]["swing_speed"]
            .as_f64()
            .unwrap();
        assert_eq!(speed, weapon_speed, "{name} is not the main hand");
        let chance = swing_chance(&prepared, name);
        assert!(
            (chance - ppm * speed / 60.0).abs() < 1e-9,
            "{name}: {chance} at {speed}"
        );
    }
}

/// Toxic Revenger (9453) 1.8, Meteor Shard (6220) 1.55, Bite of Serra'kis (6904) 1.45, and
/// Stinging Viper (6472) 1.75 in place of the reference's 3.2, each by its own speed.
#[test]
fn measured_one_hand_rates() {
    for (id, name, ppm, weapon_speed) in [
        (9453, "Toxic Revenger", 1.8, 1.9),
        (6220, "Meteor Shard", 1.55, 1.8),
        (6904, "Bite of Serra'kis", 1.45, 1.3),
        (6472, "Stinging Viper", 1.75, 2.8),
    ] {
        let prepared = prepared(&retribution_wearing(&[(MAIN_HAND, id)]), Rules::Forever);
        let speed = prepared["melee"]["main_hand"]["swing_speed"]
            .as_f64()
            .unwrap();
        assert_eq!(speed, weapon_speed, "{name} is not the main hand");
        let chance = swing_chance(&prepared, name);
        assert!(
            (chance - ppm * speed / 60.0).abs() < 1e-9,
            "{name}: {chance} at {speed}"
        );
    }
}

/// The reference keeps Stinging Viper's 3.2.
#[test]
fn the_reference_keeps_stinging_vipers_rate() {
    let prepared = prepared(
        &retribution_wearing(&[(MAIN_HAND, 6472)]),
        Rules::GoReference,
    );
    let speed = prepared["melee"]["main_hand"]["swing_speed"]
        .as_f64()
        .unwrap();
    let chance = swing_chance(&prepared, "Stinging Viper");
    assert!((chance - 3.2 * speed / 60.0).abs() < 1e-9, "{chance}");
}

// ---------------------------------------------------------------------------------------------
// Drakefang Butcher.
// ---------------------------------------------------------------------------------------------

fn butcher_against(mob_type: &str) -> Value {
    let mut request = retribution_wearing(&[(MAIN_HAND, 12463)]);
    for target in request["encounter"]["targets"].as_array_mut().unwrap() {
        target["mobType"] = json!(mob_type);
    }
    prepared(&request, Rules::Forever)
}

fn rend_multiplier(prepared: &Value) -> f64 {
    prepared["player"]["spells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|spell| spell["action_id"]["spell_id"] == 14118)
        .expect("Rend is registered")["damage_multiplier"]
        .as_f64()
        .unwrap()
}

/// "Deals 2 times as much damage to Dragonkin": the row's second effect (SpellEffect 1324263,
/// a dummy of 2) doubles the bleed's multiplier against a Dragonkin target.
#[test]
fn drakefang_rend_is_doubled_against_dragonkin() {
    let dragonkin = rend_multiplier(&butcher_against("MobTypeDragonkin"));
    let humanoid = rend_multiplier(&butcher_against("MobTypeHumanoid"));
    assert!(
        (dragonkin / humanoid - 2.0).abs() < 1e-12,
        "{dragonkin} vs {humanoid}"
    );
}

/// Against Dragonkin the bleed's ticks deal twice what they deal against a Humanoid. The raid's
/// debuffs are left off: a flat bonus damage taken on the target is added after the factor.
#[test]
fn drakefang_rend_ticks_twice_as_hard_against_dragonkin() {
    let tick = |mob_type: &str| {
        let mut request = retribution_wearing(&[(MAIN_HAND, 12463)]);
        request["raid"]["debuffs"] = json!({});
        for target in request["encounter"]["targets"].as_array_mut().unwrap() {
            target["mobType"] = json!(mob_type);
        }
        let result = run(&prepared(&request, Rules::Forever));
        let player = &result["raidMetrics"]["parties"][0]["players"][0];
        player["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["id"]["spellId"] == 14118)
            .flat_map(|action| action["targets"].as_array().unwrap())
            .filter_map(|target| target["tickRange"]["max"].as_f64())
            .fold(0.0, f64::max)
    };
    let dragonkin = tick("MobTypeDragonkin");
    let humanoid = tick("MobTypeHumanoid");
    assert!(humanoid > 0.0);
    assert!(
        (dragonkin / humanoid - 2.0).abs() < 0.02,
        "{dragonkin} vs {humanoid}"
    );
}
