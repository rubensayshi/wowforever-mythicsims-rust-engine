//! Forever gear that refused real players' sims on 21 November 2026:
//!
//! - "no item with id 287031", three Enhancement Shaman Quick Sims: Spearfisher's Waders, mail
//!   legs, item level 58, 301 armor, +17 Agility, +19 Intellect and 0.8% dodge. Its ItemSparse row
//!   is encrypted (Item row only in client 1.60.1.71037); the row is its Wowhead Forever tooltip,
//!   with the dodge as the 10 rating Wowhead Forever's gear planner states unrounded (0.8333%).
//!   The fishing line changes nothing in a fight.
//! - "no item with id 226999", two Retribution Paladin Quick Sims: Soulforge Boots, the feet of
//!   the reissued Soulforge Armor (ItemSet 1673), item level 60, 470 armor, +12 Strength, +13
//!   Stamina, +12 Intellect, +10 Spirit, 4 mana per 5 sec and damage and healing up to 12. The
//!   ItemSparse row of client 1.60.1.71037 and the tooltip agree. The Paladin wore it with five
//!   reference pieces of the set, so it is the sixth piece.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

const SPEARFISHERS_WADERS: i64 = 287031;
const SOULFORGE_BOOTS: i64 = 226999;

const SHAMAN: &str = "fixtures/forever-gaps/enhancement-shaman-spearfisher-waders.request.json";
const PALADIN: &str = "fixtures/forever-gaps/retribution-paladin-soulforge-boots.request.json";

fn request(fixture: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(20);
    request
}

/// A fixture's request without the item `id`.
fn without(fixture: &str, id: i64) -> Value {
    let mut request = request(fixture);
    let equipment = request["raid"]["parties"][0]["players"][0]["equipment"]["items"]
        .as_array_mut()
        .unwrap();
    let at = equipment
        .iter()
        .position(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("{fixture} wears no {id}"));
    equipment.remove(at);
    request
}

fn prepared(request: &Value) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever)
        .unwrap_or_else(|err| panic!("{err}"))
}

/// A prepared input through the gate and run.
fn simulate(value: &Value) {
    let prepared: PreparedV2 = serde_json::from_value(value.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|err| panic!("{err:?}"));
    let result = simulate_prepared(&prepared).unwrap().result;
    let player = &result["raidMetrics"]["parties"][0]["players"][0];
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0);
}

fn stat(prepared: &Value, name: &str) -> f64 {
    prepared["player"]["stats"][name].as_f64().unwrap_or(0.0)
}

fn set_bonuses(prepared: &Value) -> Vec<String> {
    prepared["player"]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|aura| aura["label"].as_str())
        .filter(|label| label.starts_with("Soulforge Armor"))
        .map(str::to_string)
        .collect()
}

#[test]
fn spearfishers_waders_give_the_shaman_their_tooltip_stats() {
    let with = prepared(&request(SHAMAN));
    let bare = prepared(&without(SHAMAN, SPEARFISHERS_WADERS));
    simulate(&with);
    // Agility, Intellect and armor pass through the character's percent bonuses (Blessing of
    // Kings, talents), and the dodge rating carries the Agility's dodge beside the item's 10.
    assert!(stat(&with, "DodgeRating") - stat(&bare, "DodgeRating") > 10.0);
    assert!((17.0..=20.0).contains(&(stat(&with, "Agility") - stat(&bare, "Agility"))));
    assert!((19.0..=23.0).contains(&(stat(&with, "Intellect") - stat(&bare, "Intellect"))));
    assert!(stat(&with, "Armor") - stat(&bare, "Armor") >= 301.0);
}

#[test]
fn soulforge_boots_are_the_paladins_sixth_soulforge_armor_piece() {
    let with = prepared(&request(PALADIN));
    let bare = prepared(&without(PALADIN, SOULFORGE_BOOTS));
    simulate(&with);
    assert_eq!(
        set_bonuses(&with),
        [
            "Soulforge Armor 2P",
            "Soulforge Armor 4P",
            "Soulforge Armor 6P"
        ]
    );
    assert_eq!(
        set_bonuses(&bare),
        ["Soulforge Armor 2P", "Soulforge Armor 4P"]
    );
    // Damage and healing up to 12 and 4 mana per 5 sec, beside what the Paladin draws from the
    // boots' Strength, Intellect and Spirit.
    for (name, item) in [("SpellDamage", 12.0), ("HealingPower", 12.0), ("MP5", 4.0)] {
        assert!(stat(&with, name) - stat(&bare, name) >= item, "{name}");
    }
}
