//! The live client's Tier 0.5 caster sets under Forever's rules, read on 23 November 2026 from
//! the ItemSetSpell rows of build 1.60.1.71037 (wago.tools, `product=wowf`) beside the Wowhead
//! Forever tooltips of Sorcerer's Bindings (226929) and Deathmist Belt (226905):
//!
//! - Sorcerer's Regalia (1671, Mage), Deathmist Raiment (1681, Warlock) and Vestments of the
//!   Virtuous (1675, Priest): (2) +8 All Resistances, (3) 8 mana per 5 sec, (4) Sudden Insight
//!   (Sorcerer's, Virtuous) or Dark Reward (Deathmist) on 5% of spell casts, (5) a crowd control
//!   proc when struck, (6) 23 spell damage and healing.
//!
//! A player reported on 23 November 2026 that the Sorcerer's set bonuses were wrong: the engine
//! registered none of the three sets, so their pieces counted for their own stats alone.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

const HEAD: usize = 0;
const SHOULDER: usize = 2;
const CHEST: usize = 4;
const WRIST: usize = 5;
const HANDS: usize = 6;
const WAIST: usize = 7;
const LEGS: usize = 8;
const FEET: usize = 9;

/// The armor slots the tests fill, emptied first so a difference is the pieces' alone.
const ARMOR_SLOTS: [usize; 8] = [HEAD, SHOULDER, CHEST, WRIST, HANDS, WAIST, LEGS, FEET];

const SORCERERS: [(usize, i64); 8] = [
    (WRIST, 226929),
    (HANDS, 226930),
    (FEET, 226931),
    (CHEST, 226932),
    (LEGS, 226933),
    (WAIST, 226934),
    (HEAD, 226935),
    (SHOULDER, 226936),
];

const DEATHMIST: [(usize, i64); 8] = [
    (WAIST, 226905),
    (CHEST, 226906),
    (WRIST, 226907),
    (FEET, 226908),
    (HEAD, 226909),
    (LEGS, 226910),
    (HANDS, 226911),
    (SHOULDER, 226912),
];

/// Vestments of the Virtuous, the two role variants mixed: Robe, Skirt and Crown of one, Wraps,
/// Cord, Epaulets, Hands and Slippers of the other.
const VIRTUOUS: [(usize, i64); 8] = [
    (CHEST, 226945),
    (WRIST, 226953),
    (WAIST, 226954),
    (SHOULDER, 226955),
    (LEGS, 226946),
    (HANDS, 226958),
    (HEAD, 226947),
    (FEET, 226959),
];

const ARCANE_MAGE: &str = "fixtures/dev-notes-20261102/arcane-mage.request.json";
const WARLOCK: &str = "fixtures/forever-gaps/demonology-warlock-black-book.request.json";
const SHADOW_PRIEST: &str = "fixtures/forever-gaps/shadow-priest-chastise.request.json";
const HOLY_PRIEST: &str = "fixtures/healers/holy-priest.request.json";

/// A fixture's request in these armor pieces alone.
fn dressed(fixture: &str, items: &[(usize, i64)]) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(20);
    request["simOptions"]["debugFirstIteration"] = json!(false);
    let player = &mut request["raid"]["parties"][0]["players"][0];
    let equipment = player["equipment"]["items"].as_array_mut().unwrap();
    equipment.resize(17, json!({}));
    for slot in ARMOR_SLOTS {
        equipment[slot] = json!({});
    }
    for &(slot, id) in items {
        equipment[slot] = json!({ "id": id });
    }
    request
}

fn prepare(request: &Value) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever)
        .unwrap_or_else(|err| panic!("{err}"))
}

/// A prepared input through the gate and run.
fn simulate(prepared: &Value) {
    let input: PreparedV2 = serde_json::from_value(prepared.clone()).unwrap();
    check_prepared(&input).unwrap_or_else(|err| panic!("{err:?}"));
    let result = simulate_prepared(&input).unwrap().result;
    let player = &result["raidMetrics"]["parties"][0]["players"][0];
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0);
}

fn stat(prepared: &Value, name: &str) -> f64 {
    prepared["player"]["stats"][name].as_f64().unwrap_or(0.0)
}

fn has_aura(prepared: &Value, label: &str) -> bool {
    prepared["player"]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .any(|aura| aura["label"] == label)
}

fn effect<'a>(prepared: &'a Value, kind: &str, trigger_aura: &str) -> Option<&'a Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == kind && effect["trigger_aura"] == trigger_aura)
}

/// What the bonus at `pieces` adds to a stat: the step from one piece fewer, less the last
/// piece's own stats.
fn bonus(fixture: &str, set: &[(usize, i64)], pieces: usize, name: &str) -> f64 {
    let with = prepare(&dressed(fixture, &set[..pieces]));
    let without = prepare(&dressed(fixture, &set[..pieces - 1]));
    let last = prepare(&dressed(fixture, &set[pieces - 1..pieces]));
    let naked = prepare(&dressed(fixture, &[]));
    (stat(&with, name) - stat(&without, name)) - (stat(&last, name) - stat(&naked, name))
}

/// Within half a point: a wearer's talents scale some stats by a percentage (a Shadow Priest
/// turns a share of its Spirit into spell damage), which moves the step a little beside the
/// bonus. Every bonus here is 8 or 23, so a missing or doubled one is far outside.
fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.5, "{actual} != {expected}");
}

/// The stat bonuses every one of the three sets shares, on one of its wearers.
fn assert_shared_stat_bonuses(fixture: &str, set: &[(usize, i64)]) {
    assert_close(bonus(fixture, set, 2, "FireResistance"), 8.0);
    assert_close(bonus(fixture, set, 3, "MP5"), 8.0);
    assert_close(bonus(fixture, set, 6, "SpellDamage"), 23.0);
    assert_close(bonus(fixture, set, 6, "HealingPower"), 23.0);
    // Nothing at the counts without a bonus.
    assert_close(bonus(fixture, set, 7, "SpellDamage"), 0.0);
    assert_close(bonus(fixture, set, 7, "MP5"), 0.0);
}

/// The Sudden Insight a set's 4 pieces describe: 200 mana on 5% of the spell casts.
fn assert_sudden_insight(prepared: &Value, aura: &str) {
    let insight = effect(prepared, "mana_on_cast", aura).expect("Sudden Insight");
    assert_eq!(insight["proc_chance"], 0.05);
    let spells = insight["spells"].as_array().unwrap();
    assert!(!spells.is_empty());
    assert!(spells.iter().all(|spell| spell["mana"] == 200.0));
}

#[test]
fn sorcerers_regalia_applies_its_bonuses_on_a_mage() {
    let six = prepare(&dressed(ARCANE_MAGE, &SORCERERS[..6]));
    for pieces in 2..=6 {
        assert!(
            has_aura(&six, &format!("Sorcerer's Regalia {pieces}P")),
            "{pieces}P"
        );
    }
    assert_shared_stat_bonuses(ARCANE_MAGE, &SORCERERS);

    let three = prepare(&dressed(ARCANE_MAGE, &SORCERERS[..3]));
    assert!(effect(&three, "mana_on_cast", "Sorcerer's Regalia 4P").is_none());
    let four = prepare(&dressed(ARCANE_MAGE, &SORCERERS[..4]));
    assert_sudden_insight(&four, "Sorcerer's Regalia 4P");
    simulate(&four);
    simulate(&six);
}

#[test]
fn deathmist_raiment_applies_its_bonuses_on_a_warlock() {
    let six = prepare(&dressed(WARLOCK, &DEATHMIST[..6]));
    for pieces in 2..=6 {
        assert!(
            has_aura(&six, &format!("Deathmist Raiment {pieces}P")),
            "{pieces}P"
        );
    }
    assert_shared_stat_bonuses(WARLOCK, &DEATHMIST);

    let three = prepare(&dressed(WARLOCK, &DEATHMIST[..3]));
    assert!(effect(&three, "spell_data_heal_proc", "Deathmist Raiment 4P").is_none());
    let four = prepare(&dressed(WARLOCK, &DEATHMIST[..4]));
    let reward =
        effect(&four, "spell_data_heal_proc", "Deathmist Raiment 4P").expect("Dark Reward");
    assert_eq!(reward["proc_chance"], 0.05);
    simulate(&four);
    simulate(&six);
}

/// The two role variants count together.
#[test]
fn vestments_of_the_virtuous_applies_its_bonuses_on_a_shadow_priest() {
    let six = prepare(&dressed(SHADOW_PRIEST, &VIRTUOUS[..6]));
    for pieces in 2..=6 {
        assert!(
            has_aura(&six, &format!("Vestments of the Virtuous {pieces}P")),
            "{pieces}P"
        );
    }
    assert_shared_stat_bonuses(SHADOW_PRIEST, &VIRTUOUS);
    assert_sudden_insight(&six, "Vestments of the Virtuous 4P");
    simulate(&six);
}

/// A healer reads the set through the client set alone: the 3 and 6 piece bonuses count once,
/// and the 4 piece mana proc is described.
#[test]
fn vestments_of_the_virtuous_counts_once_on_a_healer() {
    assert_close(bonus(HOLY_PRIEST, &VIRTUOUS, 3, "MP5"), 8.0);
    assert_close(bonus(HOLY_PRIEST, &VIRTUOUS, 6, "HealingPower"), 23.0);
    let four = prepare(&dressed(HOLY_PRIEST, &VIRTUOUS[..4]));
    assert_sudden_insight(&four, "Vestments of the Virtuous 4P");
}
