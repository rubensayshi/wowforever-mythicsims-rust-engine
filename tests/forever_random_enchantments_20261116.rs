//! The Forever client rolls its random enchantments from item bonus lists, with names and stats
//! the reference database's suffixes do not have ("of the Sorcerer"). A request states the bonus
//! list as the item's random suffix, and Forever's rules read its stats from the item's random
//! property points (`src/prepare/forever_random_enchantments.rs`). Reported on Discord on 15
//! November 2026: Potent Boots could not be given "of the Sorcerer".

use forever_engine::{prepare_json_with, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// The turret Hunter fixture with no talents, buffs, debuffs or consumables, wearing only the
/// given item with the given random suffix.
fn request(item: i64, suffix: i64) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/prepull/hunter-turret-aimed-shot.request.json");
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(1);
    request["raid"]["buffs"] = json!({});
    request["raid"]["debuffs"] = json!({});
    request["raid"]["parties"][0]["buffs"] = json!({});
    let player = &mut request["raid"]["parties"][0]["players"][0];
    player["talentsString"] = json!("");
    player["buffs"] = json!({});
    player["consumables"] = json!({});
    let item = if suffix == 0 {
        json!({ "id": item })
    } else {
        json!({ "id": item, "randomSuffix": suffix })
    };
    player["equipment"] = json!({ "items": [item] });
    serde_json::to_vec(&request).unwrap()
}

fn stats(item: i64, suffix: i64, rules: Rules) -> Result<Value, String> {
    prepare_json_with(&request(item, suffix), "rust", rules)
        .map(|prepared| prepared["player"]["stats"].clone())
        .map_err(|err| err.to_string())
}

/// What the suffix adds to a stat of the character sheet.
fn added(item: i64, suffix: i64, stat: &str) -> f64 {
    let without = stats(item, 0, Rules::Forever).unwrap();
    let with = stats(item, suffix, Rules::Forever).unwrap();
    with[stat].as_f64().unwrap() - without[stat].as_f64().unwrap()
}

#[test]
fn potent_boots_of_the_sorcerer_add_their_tooltip() {
    // "+7 Stamina, +12 Intellect, Equip: Increases damage and healing done by magical spells and
    // effects by up to 6." (Wowhead Forever, item 15171 with bonus 12745.)
    assert_eq!(added(15171, 12745, "SpellDamage"), 6.0);
    assert_eq!(added(15171, 12745, "HealingPower"), 6.0);
    // "of Power": +32 Attack Power, which is ranged attack power too.
    assert_eq!(added(15171, 12733, "RangedAttackPower"), 32.0);
}

#[test]
fn a_roll_the_item_does_not_have_is_refused() {
    let refused = stats(15171, 12906, Rules::Forever).unwrap_err();
    assert!(
        refused.contains("does not roll random enchantment 12906"),
        "{refused}"
    );
}

#[test]
fn the_reference_rules_know_no_client_roll() {
    let refused = stats(15171, 12745, Rules::GoReference).unwrap_err();
    assert!(
        refused.contains("no random suffix with id 12745"),
        "{refused}"
    );
    // A suffix of the reference database still reads under both rules.
    assert!(stats(15171, 611, Rules::Forever).is_ok());
    assert!(stats(15171, 611, Rules::GoReference).is_ok());
}
