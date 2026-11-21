//! The 68 crafted Tier 1 pieces (273884 to 273951: Grovekeeper, Wildstalker, Manaflare,
//! Justice, Conviction, Grimstitch, Spiritcaller, Demonheart and Glory legs, hands, shoulders and
//! feet), from Wowhead's Forever tooltips of 22 November 2026, under Forever's rules only.
//!
//! The reference rows are armor alone at item level 61. The hotfixes of 20 November gave the
//! pieces their stats, and the tooltips state them at item level 60: Manaflare Pants (273904)
//! "81 Armor", "+16 Stamina", "+19 Intellect", "+9 Spirit", "+9 Nature Resistance", "Equip:
//! Increases damage and healing done by magical spells and effects by up to 20." and "Equip: Your
//! spells pierce 5 Magical Resistance."; Grimstitch Pants (273928) "+16 Strength", "+19 Agility",
//! "+19 Stamina" and "Equip: Improves your chance to hit by 1.5%."; Legplates of Glory (273944)
//! "+22 Strength", "+15 Agility", "+23 Stamina" and 0.9% hit. `tools/rust_live_items.py` wrote
//! the live rows from the tooltips.
//!
//! Each piece is "Unique-Equipped: Artisan's Tier (1)" (limit category 708), one crafted piece a
//! character. The sets now list six items each, two of them ids with no public tooltip, so no
//! set bonus beyond the pieces' own count is reachable.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};

const FIRST: i64 = 273884;
const LAST: i64 = 273951;
const MANAFLARE_PANTS: i64 = 273904;
const DEMONHEART_LEGGINGS: i64 = 273940;
const GRIMSTITCH_PANTS: i64 = 273928;
const LEGPLATES_OF_GLORY: i64 = 273944;
const LEGS: usize = 8;

#[path = "support/rogue_113483.rs"]
mod rogue_113483;
#[path = "support/shaman_layout.rs"]
mod shaman_layout;
#[path = "support/warrior_trees_20261104.rs"]
mod warrior_trees_20261104;

fn rows(path: &str) -> BTreeMap<i64, Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|row| (FIRST..=LAST).contains(&row["id"].as_i64().unwrap()))
        .map(|row| (row["id"].as_i64().unwrap(), row))
        .collect()
}

fn player(request: &mut Value) -> &mut Value {
    &mut request["raid"]["parties"][0]["players"][0]
}

/// A fixture's request (a path under `fixtures/`) without buffs, consumables or an item swap,
/// with this item in the legs slot (0 for none).
fn dressed(fixture: &str, legs: i64) -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("fixtures/{fixture}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    // The recorded talents in the order Forever's rules read (as `tests/forever_gaps.rs` does).
    rogue_113483::forever_request(&mut request);
    warrior_trees_20261104::forever_request(&mut request);
    shaman_layout::forever_request(&mut request);
    request["simOptions"]["iterations"] = json!(20);
    request["raid"]["buffs"] = json!({});
    request["raid"]["parties"][0]["buffs"] = json!({});
    let player = player(&mut request);
    player["buffs"] = json!({});
    player["consumables"] = json!({});
    let fields = player.as_object_mut().unwrap();
    fields.remove("itemSwap");
    fields.insert("enableItemSwap".to_string(), json!(false));
    let equipment = player["equipment"]["items"].as_array_mut().unwrap();
    equipment.resize(17, json!({}));
    equipment[LEGS] = if legs == 0 {
        json!({})
    } else {
        json!({ "id": legs })
    };
    request
}

fn prepare(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err}"))
}

fn stat(prepared: &Value, name: &str) -> f64 {
    prepared["player"]["stats"][name].as_f64().unwrap_or(0.0)
}

fn gain(with: &Value, without: &Value, name: &str) -> f64 {
    stat(with, name) - stat(without, name)
}

/// A primary stat passes through the character's percent bonuses, so an item's N arrives as N
/// to a fifth more.
fn assert_primary(with: &Value, without: &Value, name: &str, value: f64) {
    let gained = gain(with, without, name);
    assert!(
        (value - 1e-9..value * 1.2).contains(&gained),
        "{name}: {gained} for an item's {value}"
    );
}

fn simulate(value: &Value) -> f64 {
    let prepared: PreparedV2 = serde_json::from_value(value.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|err| panic!("{err:?}"));
    let result = simulate_prepared(&prepared).unwrap().result;
    result["raidMetrics"]["parties"][0]["players"][0]["dps"]["avg"]
        .as_f64()
        .unwrap()
}

/// Every piece has a live row at item level 60 with stats beyond its armor; each keeps its
/// reference row's name, set, class and limit category.
#[test]
fn every_crafted_piece_has_a_live_row() {
    let reference = rows("data/items.jsonl");
    let live = rows("data/forever-live-items.jsonl");
    assert_eq!(reference.len(), 68);
    assert_eq!(
        live.keys().collect::<Vec<_>>(),
        reference.keys().collect::<Vec<_>>()
    );
    for (id, row) in &live {
        let base = &row["scaling_options"]["0"];
        assert_eq!(base["ilvl"], 60, "{id}");
        let stats = base["stats"].as_object().unwrap();
        assert!(
            stats
                .iter()
                .any(|(stat, value)| !["30", "31"].contains(&stat.as_str())
                    && value.as_f64().unwrap() > 0.0),
            "{id} has stats"
        );
        for field in ["name", "set_id", "class_allowlist", "limit_category"] {
            assert_eq!(row[field], reference[id][field], "{id} {field}");
        }
        assert_eq!(row["limit_category"], 708, "{id}");
    }
}

/// Manaflare Pants on an Arcane Mage: the tooltip's Stamina, Intellect and 20 spell damage and
/// healing, and the sim runs.
#[test]
fn manaflare_pants_give_their_tooltip_stats() {
    let fixture = "dev-notes-20261102/arcane-mage";
    let bare = prepare(&dressed(fixture, 0), Rules::Forever);
    let with = prepare(&dressed(fixture, MANAFLARE_PANTS), Rules::Forever);
    assert_primary(&with, &bare, "Stamina", 16.0);
    assert_primary(&with, &bare, "Intellect", 19.0);
    assert_eq!(gain(&with, &bare, "SpellDamage"), 20.0);
    assert_eq!(gain(&with, &bare, "HealingPower"), 20.0);
    assert_eq!(gain(&with, &bare, "SpellPiercing"), 5.0);
    assert!(simulate(&with) > simulate(&bare));
}

/// Demonheart Leggings on a Demonology Warlock: 20 spell damage and 1.1% hit.
#[test]
fn demonheart_leggings_give_their_tooltip_stats() {
    let fixture = "forever-gaps/demonology-warlock-pet-values";
    let bare = prepare(&dressed(fixture, 0), Rules::Forever);
    let with = prepare(&dressed(fixture, DEMONHEART_LEGGINGS), Rules::Forever);
    assert_primary(&with, &bare, "Intellect", 19.0);
    assert_eq!(gain(&with, &bare, "SpellDamage"), 20.0);
    assert!(gain(&with, &bare, "SpellHitPercent") > 0.0);
}

/// Grimstitch Pants on a Combat Rogue and Legplates of Glory on an Arms Warrior: Strength,
/// Agility and hit, where the reference rows gave armor alone.
#[test]
fn the_melee_legs_give_their_tooltip_stats() {
    for (fixture, legs, strength, agility) in [
        (
            "forever-gaps/combat-rogue-darkmantle-riposte",
            GRIMSTITCH_PANTS,
            16.0,
            19.0,
        ),
        (
            "forever-gaps/arms-warrior-revenge",
            LEGPLATES_OF_GLORY,
            22.0,
            15.0,
        ),
    ] {
        let bare = prepare(&dressed(fixture, 0), Rules::Forever);
        let with = prepare(&dressed(fixture, legs), Rules::Forever);
        assert_primary(&with, &bare, "Strength", strength);
        assert_primary(&with, &bare, "Agility", agility);
        assert!(gain(&with, &bare, "PhysicalHitPercent") > 0.0, "{legs}");
    }
}
