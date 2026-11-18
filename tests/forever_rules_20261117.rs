//! Forever's rules on 17 November 2026: an Elemental Shaman's rotation can cast Nature's
//! Swiftness (16188). Client 16188 makes the next Nature spell with a cast time under 10 sec
//! instant; its aura lasts until a cast spends it (DurationIndex 21, one charge), the same as a
//! Restoration healer's.

use forever_engine::{check_prepared, prepare_json_with, simulate_prepared, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// 53/0/18 with Nature's Swiftness, from a prod sim that the gate refused on 16 Nov.
const ELEMENTAL_WITH_SWIFTNESS: &str = "530033102310135--05305100104101";
const NATURES_SWIFTNESS: i64 = 16188;
/// Lightning Bolt X.
const LIGHTNING_BOLT: i64 = 15208;

fn request() -> Value {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/mage/prepared-v2"
    ))
    .join("production-elemental-shaman.request.json");
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let player = &mut request["raid"]["parties"][0]["players"][0];
    player["talentsString"] = json!(ELEMENTAL_WITH_SWIFTNESS);
    player["rotation"] = json!({"type": "TypeAPL", "priorityList": [
        {"action": {"castSpell": {"spellId": {"spellId": NATURES_SWIFTNESS}}}},
        {"action": {"castSpell": {"spellId": {"spellId": LIGHTNING_BOLT}}}},
    ]});
    request["simOptions"]["iterations"] = json!(20);
    request
}

fn casts_after(result: &Value, marker: &str) -> Vec<String> {
    let log = result["logs"].as_str().unwrap();
    let (_, rest) = log
        .split_once(marker)
        .unwrap_or_else(|| panic!("the first fight never logs {marker}"));
    rest.lines()
        .filter(|line| line.contains("Casting"))
        .take(2)
        .map(str::to_string)
        .collect()
}

#[test]
fn elemental_natures_swiftness_makes_the_next_lightning_bolt_instant() {
    let prepared =
        prepare_json_with(request().to_string().as_bytes(), "rust", Rules::Forever).unwrap();
    let auras: Vec<&Value> = prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|effect| effect["kind"] == "instant_cast_aura")
        .collect();
    assert_eq!(auras.len(), 1);
    assert_eq!(auras[0]["spell_id"], NATURES_SWIFTNESS);
    assert!(!auras[0]["changes"].as_array().unwrap().is_empty());

    let prepared = serde_json::from_value(prepared).unwrap();
    check_prepared(&prepared).expect("the gate takes a rotation that casts Nature's Swiftness");
    let result = simulate_prepared(&prepared).unwrap().result;
    let next = casts_after(&result, "Completed cast {SpellID: 16188}");
    assert!(
        next[0].contains("{SpellID: 15208}") && next[0].contains("Cast Time = 0s"),
        "{next:?}"
    );
    // Spent: the bolt after it takes its cast time again.
    assert!(!next[1].contains("Cast Time = 0s"), "{next:?}");
}

/// Without the talent nothing changes: no instant cast aura in the export.
#[test]
fn elemental_without_natures_swiftness_exports_no_instant_cast() {
    let mut request = request();
    request["raid"]["parties"][0]["players"][0]["talentsString"] =
        json!("5502331020021351-05-05305");
    request["raid"]["parties"][0]["players"][0]["rotation"] = json!({"type": "TypeAPL",
        "priorityList": [{"action": {"castSpell": {"spellId": {"spellId": LIGHTNING_BOLT}}}}]});
    let prepared =
        prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever).unwrap();
    assert!(!prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|effect| effect["kind"] == "instant_cast_aura"));
}
