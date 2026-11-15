//! Reference items whose live rows kept a Classic stat their Wowhead Forever tooltip no longer
//! states, from the tooltips of 15 November 2026 (`tools/rust_live_items.py`, the rule for the
//! stats a tooltip states only in an "Equip:" line):
//!
//! - Royal Seal of Eldre'Thalas (18473), the Hunter's: "+10 Agility", "+15 Nature Resistance",
//!   "Equip: +45 Attack Power against Demons." The live row kept the reference's 48 ranged attack
//!   power, which every fight took. The 45 against Demons goes to
//!   `data/forever-live-item-bonuses.jsonl` as melee attack power against Demons, as
//!   `tools/rust_client_items.py` reads such a line.
//! - Burst of Knowledge (11832): "Restores +8 mana per 5 sec." and its use. The row kept 12
//!   healing and 12 spell damage.
//! - Verdant Footpads (13954): 38 healing and 12 damage, 5 mana per 5 sec, and "Equip: Increases
//!   damage done to Undead by magical spells and effects by up to 21." The row kept 24 Nature
//!   damage; the 21 against Undead is a bonus row.
//! - Atal'ai Gloves (10787) and Drakestone (10796): "Increases damage done to Humanoids
//!   (Dragonkin) by magical spells and effects by up to 14 (11)." The reference rows hold 9 and 7
//!   healing and spell damage against every target; the live rows hold none, and the bonuses are
//!   bonus rows.
//! - Sang'thraze the Deflector (9379): the parry is the chance on hit's 2% for 9 sec alone; the
//!   row kept a static 15 parry rating.
//! - Vigilance Charm (18370): "+10 Strength" and its use; the row kept 24 dodge rating.
//! - Shroud of Arcane Mastery (22330): Stamina, Intellect and "Your spells pierce 4 Magical
//!   Resistance."; the row kept 10 spell hit rating.
//!
//! The Royal Seal's other class versions state bonuses against a creature type their rows never
//! held: the Rogue's 45 attack power against Humanoids (18465) and the Warlock's and Shaman's 18
//! spell damage against Demons (18467, 18471). They are bonus rows too.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

const ROYAL_SEAL_HUNTER: i64 = 18473;
const DEVILSAUR_TOOTH: i64 = 19992;
const TRINKET: usize = 13;

fn fixture(path: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(20);
    request
}

fn player(request: &mut Value) -> &mut Value {
    &mut request["raid"]["parties"][0]["players"][0]
}

/// The Marksmanship Hunter, who wears the Royal Seal in the second trinket slot, with this item
/// there instead (0 for none), against targets of this creature type (None for the fixture's).
fn hunter(trinket: i64, mob_type: Option<&str>) -> Value {
    let mut request =
        fixture("fixtures/forever-gaps/marksmanship-hunter-sequence-conditions.request.json");
    let items = player(&mut request)["equipment"]["items"]
        .as_array_mut()
        .unwrap();
    items[TRINKET] = if trinket == 0 {
        json!({})
    } else {
        json!({ "id": trinket })
    };
    if let Some(mob_type) = mob_type {
        for target in request["encounter"]["targets"].as_array_mut().unwrap() {
            target["mobType"] = json!(mob_type);
        }
    }
    request
}

/// The holy paladin fixture with no talents wearing only this item in its slot (no class
/// allowlist holds any of the items it wears here), against targets of this creature type.
fn paladin(item: i64, mob_type: Option<&str>) -> Value {
    let mut request = fixture("fixtures/mage/prepared-v2/holy-paladin-healing.request.json");
    let player = player(&mut request);
    player["talentsString"] = json!("");
    let mut items = vec![json!({}); 17];
    if item != 0 {
        items[slot(item)] = json!({ "id": item });
    }
    player["equipment"] = json!({ "items": items });
    if let Some(mob_type) = mob_type {
        for target in request["encounter"]["targets"].as_array_mut().unwrap() {
            target["mobType"] = json!(mob_type);
        }
    }
    request
}

/// The equipment slot of each item the paladin wears.
fn slot(item: i64) -> usize {
    match item {
        22330 => 3,
        10787 => 6,
        13954 => 9,
        11832 | 18370 => 12,
        9379 => 14,
        10796 => 15,
        _ => panic!("no slot for {item}"),
    }
}

fn prepared(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err}"))
}

fn stat(prepared: &Value, name: &str) -> f64 {
    prepared["player"]["stats"][name].as_f64().unwrap_or(0.0)
}

/// What an item adds to a stat over the same character without it.
fn added(with: &Value, without: &Value, name: &str) -> f64 {
    stat(with, name) - stat(without, name)
}

/// The `mob_type_bonus` effect of an item's aura, if the prepared input has one.
fn bonus<'a>(prepared: &'a Value, aura: &str) -> Option<&'a Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "mob_type_bonus" && effect["aura"] == aura)
}

fn simulate(value: &Value) -> Value {
    let prepared: PreparedV2 = serde_json::from_value(value.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|err| panic!("{err:?}"));
    let result = simulate_prepared(&prepared).unwrap().result;
    result["raidMetrics"]["parties"][0]["players"][0].clone()
}

// ---------------------------------------------------------------------------------------------
// Royal Seal of Eldre'Thalas, the Hunter's.
// ---------------------------------------------------------------------------------------------

/// The seal's ranged attack power is its 10 Agility's alone, as Devilsaur Tooth's 7 Agility's is;
/// the reference's rules keep the reference row's 48.
#[test]
fn the_hunters_seal_gives_no_ranged_attack_power_of_its_own() {
    let without = prepared(&hunter(0, None), Rules::Forever);
    let seal = prepared(&hunter(ROYAL_SEAL_HUNTER, None), Rules::Forever);
    let tooth = prepared(&hunter(DEVILSAUR_TOOTH, None), Rules::Forever);
    let per_agility = |with: &Value| {
        added(with, &without, "RangedAttackPower") / added(with, &without, "Agility")
    };
    assert!(added(&seal, &without, "Agility") > 0.0);
    assert!((per_agility(&seal) - per_agility(&tooth)).abs() < 0.05);
    assert_eq!(
        added(&seal, &without, "AttackPower"),
        added(&seal, &without, "Agility")
    );
    assert_eq!(added(&seal, &without, "NatureResistance"), 15.0);
    simulate(&seal);

    let without = prepared(&hunter(0, None), Rules::GoReference);
    let seal = prepared(&hunter(ROYAL_SEAL_HUNTER, None), Rules::GoReference);
    assert_eq!(added(&seal, &without, "RangedAttackPower"), 48.0);
}

/// Against Demons the seal adds 45 melee attack power; against any other type, or none, nothing.
#[test]
fn the_hunters_seal_adds_attack_power_against_demons_alone() {
    let demons = prepared(
        &hunter(ROYAL_SEAL_HUNTER, Some("MobTypeDemon")),
        Rules::Forever,
    );
    let effect = bonus(&demons, "Royal Seal of Eldre'Thalas").expect("the bonus against Demons");
    assert_eq!(effect["melee_attack_power"], 45.0);
    assert_eq!(effect["attack_power"], 0.0);
    assert_eq!(effect["ranged_attack_power"], 0.0);
    assert_eq!(effect["spell_damage"], 0.0);
    simulate(&demons);
    for mob_type in [None, Some("MobTypeUndead"), Some("MobTypeHumanoid")] {
        let other = prepared(&hunter(ROYAL_SEAL_HUNTER, mob_type), Rules::Forever);
        assert!(
            bonus(&other, "Royal Seal of Eldre'Thalas").is_none(),
            "{mob_type:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The other items.
// ---------------------------------------------------------------------------------------------

/// Each item adds none of the stats its row kept, under Forever's rules, and the reference's rows
/// still add them.
#[test]
fn the_rows_no_longer_hold_the_stats_their_tooltips_do_not_state() {
    let cases: [(i64, &[&str], &[f64]); 7] = [
        (11832, &["HealingPower", "SpellDamage"], &[12.0, 12.0]),
        (13954, &["NatureDamage"], &[24.0]),
        (10787, &["HealingPower", "SpellDamage"], &[9.0, 9.0]),
        (10796, &["HealingPower", "SpellDamage"], &[7.0, 7.0]),
        (9379, &["ParryRating"], &[15.0]),
        (18370, &["DodgeRating"], &[24.0]),
        (22330, &["SpellHitRating"], &[10.0]),
    ];
    let forever = prepared(&paladin(0, None), Rules::Forever);
    let reference = prepared(&paladin(0, None), Rules::GoReference);
    for (id, stats, held) in cases {
        let with = prepared(&paladin(id, None), Rules::Forever);
        let reference_with = prepared(&paladin(id, None), Rules::GoReference);
        for (name, value) in stats.iter().zip(held) {
            assert_eq!(added(&with, &forever, name), 0.0, "item {id} {name}");
            assert!(
                added(&reference_with, &reference, name) > value - 0.01,
                "item {id} {name} under the reference's rules"
            );
        }
    }
}

/// The kept stats stay: Burst of Knowledge's 8 mana per 5 sec and Verdant Footpads' 38 healing,
/// 12 spell damage and 5 mana per 5 sec.
#[test]
fn the_stated_stats_stay() {
    let without = prepared(&paladin(0, None), Rules::Forever);
    let burst = prepared(&paladin(11832, None), Rules::Forever);
    assert_eq!(added(&burst, &without, "MP5"), 8.0);
    let footpads = prepared(&paladin(13954, None), Rules::Forever);
    assert_eq!(added(&footpads, &without, "HealingPower"), 38.0);
    assert_eq!(added(&footpads, &without, "SpellDamage"), 12.0);
    assert_eq!(added(&footpads, &without, "MP5"), 5.0);
}

/// The spell damage against a creature type applies against that type alone.
#[test]
fn spell_damage_against_a_creature_type_applies_against_it_alone() {
    for (id, name, mob_type, value) in [
        (13954, "Verdant Footpads", "MobTypeUndead", 21.0),
        (10787, "Atal'ai Gloves", "MobTypeHumanoid", 14.0),
        (10796, "Drakestone", "MobTypeDragonkin", 11.0),
    ] {
        let versus = prepared(&paladin(id, Some(mob_type)), Rules::Forever);
        let effect = bonus(&versus, name).unwrap_or_else(|| panic!("{name} against {mob_type}"));
        assert_eq!(effect["spell_damage"], value, "{name}");
        simulate(&versus);
        let other = prepared(&paladin(id, Some("MobTypeBeast")), Rules::Forever);
        assert!(bonus(&other, name).is_none(), "{name} against Beasts");
    }
}

/// The bonus rows of the Royal Seal's other class versions, as their tooltips state them.
#[test]
fn the_other_royal_seals_state_their_creature_type_bonuses() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/forever-live-item-bonuses.jsonl");
    let rows: Vec<Value> = fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let row = |id: i64| rows.iter().find(|row| row["id"] == id).unwrap().clone();
    assert_eq!(
        row(18465)["melee_attack_power_versus"],
        json!({ "MobTypeHumanoid": 45 })
    );
    assert_eq!(
        row(18467)["spell_damage_versus"],
        json!({ "MobTypeDemon": 18 })
    );
    assert_eq!(
        row(18471)["spell_damage_versus"],
        json!({ "MobTypeDemon": 18 })
    );
    assert_eq!(
        row(ROYAL_SEAL_HUNTER)["melee_attack_power_versus"],
        json!({ "MobTypeDemon": 45 })
    );
}
