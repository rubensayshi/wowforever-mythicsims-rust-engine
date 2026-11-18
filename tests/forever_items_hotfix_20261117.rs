//! Item rows of Blizzard's WoW Forever hotfixes of 17 November 2026 and items that refused real
//! players' sims on 17 and 18 November 2026, each from Wowhead's Forever tooltip of 18 November
//! 2026 beside the client rows wago.tools serves (`product=wowf`, `build=1.60.1.70959`), under
//! Forever's rules only.
//!
//! The hotfixes:
//!
//! - "Corrected Whiteout Staff and Crackling Staff so that they have the same stats: 25 Stamina,
//!   25 Intellect, 74 Spell Power from Weapon damage." Whiteout Staff (19101) had 16 Intellect and
//!   Crackling Staff (19102) 25 Intellect and 99 spell damage and healing with no Stamina.
//! - "Soulforge Armor (upgraded Dungeon Set for tanks) now has Spell Power (was Strength)." The
//!   eight tank pieces of the reissued set (226985 to 226992, ItemSet 1673): each lost its
//!   Strength and states spell damage and healing, and its Intellect is the tooltip's. The
//!   Classic pieces (22086 to 22093) and the reissue's other sixteen read as their rows did.
//! - "Escape Plan now has correct range and damage values." The gun (287026) is 54 to 102 at
//!   2.80, where its row had 78 to 78. Its use, a Disarm (13534) at combat range, is not pressed.
//! - "Shadow Priests can now hold a Night Watchman's Torch." The torch (280612) is an item of the
//!   client's miscellaneous class with no equipment slot, lit by a use that ends in combat, so no
//!   row holds it.
//!
//! The refusals, "no item with id N":
//!
//! - Filcher's Cowl (219343), cloak, item level 50: 36 Armor, +11 Agility, +7 Spirit, and an
//!   effective stealth level no fight reads. Two refusals (Combat Rogue).
//! - Totem of Sustaining (23200), relic, item level 65: "Equip: Increases the maximum number of
//!   charges on your Water Shield by 1." ItemEffect 104176 is an equip of Increased Water Shield
//!   Charges (28856): SpellEffect 705031, a flat modifier (107) of 1 on the charges (misc 4) of
//!   Water Shield's class mask. Four refusals (Enhancement and Restoration Shaman).
//! - Weed Whacker (274270), two-hand axe, item level 57, 109 to 165, speed 3.10, +12 Strength:
//!   "Chance on hit: Assault the target with a dangerous toxin, dealing 100 to 152 Nature damage.
//!   Deals 3 times as much damage to Plants." ItemEffect 233728 is a chance on hit of a Grass
//!   Killer row of its own (1293568) with no rate and no cooldown: a School Damage of 126 with
//!   variance 0.42, Nature, on the magic table (SpellEffect 1314320, SpellMisc 840998,
//!   SpellCategories 183682). One refusal (Enhancement Shaman, 16 November).
//!
//! No refused request is copied here: the tests dress fixture characters of `fixtures/`.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[path = "support/rogue_113483.rs"]
mod rogue_113483;

const WHITEOUT_STAFF: i64 = 19101;
const CRACKLING_STAFF: i64 = 19102;
const ESCAPE_PLAN: i64 = 287026;
const FILCHERS_COWL: i64 = 219343;
const TOTEM_OF_SUSTAINING: i64 = 23200;
const WEED_WHACKER: i64 = 274270;
const WEED_WHACKER_GRASS_KILLER: i64 = 1293568;

/// The tank pieces of Soulforge Armor: slot, item, the Strength the row held before the hotfix,
/// and the Stamina, Intellect, and spell damage and healing of today's tooltip.
const SOULFORGE_TANK: [(usize, i64, f64, f64, f64, f64); 8] = [
    (WRIST, 226985, 8.0, 16.0, 8.0, 6.0),
    (WAIST, 226986, 13.0, 18.0, 13.0, 12.0),
    (SHOULDER, 226987, 12.0, 18.0, 12.0, 9.0),
    (LEGS, 226988, 16.0, 24.0, 16.0, 16.0),
    (HEAD, 226989, 18.0, 33.0, 18.0, 13.0),
    (HANDS, 226990, 11.0, 15.0, 11.0, 12.0),
    (FEET, 226991, 12.0, 26.0, 12.0, 8.0),
    (CHEST, 226992, 18.0, 29.0, 18.0, 14.0),
];

const HEAD: usize = 0;
const SHOULDER: usize = 2;
const BACK: usize = 3;
const CHEST: usize = 4;
const WRIST: usize = 5;
const HANDS: usize = 6;
const WAIST: usize = 7;
const LEGS: usize = 8;
const FEET: usize = 9;
const MAIN_HAND: usize = 14;
const OFF_HAND: usize = 15;
const RANGED: usize = 16;
/// The Hunter fixture lists its items without an off hand, so its ranged weapon is the sixteenth.
const HUNTER_RANGED: usize = 15;

fn player(request: &mut Value) -> &mut Value {
    &mut request["raid"]["parties"][0]["players"][0]
}

/// A fixture's request (a path under `fixtures/`) without buffs, consumables or an item swap,
/// with these items worn in place of the fixture's. Item 0 empties a slot.
fn dressed(fixture: &str, items: &[(usize, i64)]) -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("fixtures/{fixture}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
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
    for &(slot, id) in items {
        equipment[slot] = if id == 0 {
            json!({})
        } else {
            json!({ "id": id })
        };
    }
    request
}

/// The Combat Rogue. The fixture's talents are read in the trees Forever's rules prepare.
fn rogue(items: &[(usize, i64)]) -> Value {
    let mut request = dressed("forever-gaps/combat-rogue-self-casts-at-player", items);
    rogue_113483::forever_request(&mut request);
    request
}

/// The Elemental Shaman, a caster who can hold a staff.
fn elemental(items: &[(usize, i64)]) -> Value {
    dressed("forever-gaps/elemental-shaman-the-elements", items)
}

/// The Restoration Shaman of the healer fixtures, who has Water Shield.
fn restoration(items: &[(usize, i64)]) -> Value {
    dressed("healers/restoration-shaman", items)
}

fn protection(items: &[(usize, i64)]) -> Value {
    dressed("forever-gaps/protection-paladin-quel-serrar", items)
}

fn retribution(items: &[(usize, i64)]) -> Value {
    dressed(
        "paladin/retribution-paladin-unsealed-wrath-command-echoes",
        items,
    )
}

fn hunter(items: &[(usize, i64)]) -> Value {
    dressed(
        "forever-gaps/marksmanship-hunter-sequence-conditions",
        items,
    )
}

fn prepare(request: &Value) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever)
        .unwrap_or_else(|err| panic!("{err}"))
}

/// A prepared input through the gate and run: the player's metrics.
fn simulate(value: &Value) -> Value {
    let prepared: PreparedV2 = serde_json::from_value(value.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|err| panic!("{err:?}"));
    let result = simulate_prepared(&prepared).unwrap().result;
    result["raidMetrics"]["parties"][0]["players"][0].clone()
}

fn stat(prepared: &Value, name: &str) -> f64 {
    prepared["player"]["stats"][name].as_f64().unwrap_or(0.0)
}

/// What a stat gains from `without` to `with`.
fn gain(with: &Value, without: &Value, name: &str) -> f64 {
    stat(with, name) - stat(without, name)
}

/// A primary stat passes through the character's percent bonuses (a talent, a racial), so an
/// item's N arrives as N to a fifth more.
fn assert_primary(with: &Value, without: &Value, name: &str, value: f64) {
    let gained = gain(with, without, name);
    assert!(
        (value - 1e-9..value * 1.2).contains(&gained),
        "{name}: {gained} for an item's {value}"
    );
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

fn effect<'a>(prepared: &'a Value, kind: &str) -> &'a Value {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == kind)
        .unwrap_or_else(|| panic!("a {kind} effect"))
}

/// The kinds of the prepared effects, in order.
fn effect_kinds(prepared: &Value) -> Vec<Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|effect| effect["kind"].clone())
        .collect()
}

fn aura<'a>(prepared: &'a Value, label: &str) -> &'a Value {
    prepared["player"]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|aura| aura["label"] == label)
        .unwrap_or_else(|| panic!("aura {label}"))
}

// ---------------------------------------------------------------------------------------------
// Whiteout Staff and Crackling Staff.
// ---------------------------------------------------------------------------------------------

/// Each staff gives 25 Stamina, 25 Intellect and 74 spell damage and healing, at 102 to 154 and
/// 3.10.
#[test]
fn each_staff_has_the_hotfixed_stats() {
    let bare = prepare(&elemental(&[(MAIN_HAND, 0), (OFF_HAND, 0)]));
    for staff in [WHITEOUT_STAFF, CRACKLING_STAFF] {
        let with = prepare(&elemental(&[(MAIN_HAND, staff), (OFF_HAND, 0)]));
        assert_primary(&with, &bare, "Stamina", 25.0);
        assert_primary(&with, &bare, "Intellect", 25.0);
        assert_eq!(gain(&with, &bare, "SpellDamage"), 74.0, "{staff}");
        assert_eq!(gain(&with, &bare, "HealingPower"), 74.0, "{staff}");
        let main_hand = &with["melee"]["main_hand"];
        assert_eq!(main_hand["base_damage_min"], 102.0);
        assert_eq!(main_hand["base_damage_max"], 154.0);
        assert_eq!(main_hand["swing_speed"], 3.1);
    }
}

/// "The same stats": the two staves leave the character with one stat sheet, and the sim runs.
#[test]
fn the_two_staves_have_the_same_stats() {
    let whiteout = prepare(&elemental(&[(MAIN_HAND, WHITEOUT_STAFF), (OFF_HAND, 0)]));
    let crackling = prepare(&elemental(&[(MAIN_HAND, CRACKLING_STAFF), (OFF_HAND, 0)]));
    assert_eq!(whiteout["player"]["stats"], crackling["player"]["stats"]);
    let player = simulate(&whiteout);
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0);
}

/// The reference keeps its rows: under its rules Crackling Staff still gives 99 spell damage.
#[test]
fn the_reference_keeps_the_classic_staves() {
    let prepare = |staff: i64| {
        let request = elemental(&[(MAIN_HAND, staff), (OFF_HAND, 0)]);
        prepare_json_with(request.to_string().as_bytes(), "rust", Rules::GoReference)
            .unwrap_or_else(|err| panic!("{err}"))
    };
    let bare = prepare(0);
    assert_eq!(gain(&prepare(CRACKLING_STAFF), &bare, "SpellDamage"), 99.0);
    assert_eq!(gain(&prepare(WHITEOUT_STAFF), &bare, "SpellDamage"), 74.0);
}

// ---------------------------------------------------------------------------------------------
// Soulforge Armor, the tank pieces.
// ---------------------------------------------------------------------------------------------

/// One piece at a time, so no set bonus is in the difference: no Strength, the tooltip's Stamina
/// and Intellect, and its spell damage and healing.
#[test]
fn each_tank_soulforge_piece_gives_spell_power_and_no_strength() {
    for (slot, id, _, stamina, intellect, spell_power) in SOULFORGE_TANK {
        let bare = prepare(&protection(&[(slot, 0)]));
        let with = prepare(&protection(&[(slot, id)]));
        assert_eq!(gain(&with, &bare, "Strength"), 0.0, "{id}");
        assert_primary(&with, &bare, "Stamina", stamina);
        assert_primary(&with, &bare, "Intellect", intellect);
        assert_eq!(gain(&with, &bare, "SpellDamage"), spell_power, "{id}");
        assert_eq!(gain(&with, &bare, "HealingPower"), spell_power, "{id}");
    }
}

/// The reissue's values: each piece's Intellect is the Strength its row held before the hotfix,
/// and the eight pieces state 90 spell damage and healing between them.
#[test]
fn the_tank_pieces_total_ninety_spell_power() {
    let total: f64 = SOULFORGE_TANK.iter().map(|piece| piece.5).sum();
    assert_eq!(total, 90.0);
    for (_, id, strength, _, intellect, _) in SOULFORGE_TANK {
        assert_eq!(strength, intellect, "{id}");
    }
}

/// All eight worn: the set's bonuses still prepare, and the Protection Paladin runs.
#[test]
fn the_tank_set_prepares_and_runs() {
    let pieces: Vec<(usize, i64)> = SOULFORGE_TANK
        .iter()
        .map(|piece| (piece.0, piece.1))
        .collect();
    let with = prepare(&protection(&pieces));
    let player = simulate(&with);
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0);
}

// ---------------------------------------------------------------------------------------------
// Escape Plan.
// ---------------------------------------------------------------------------------------------

/// The gun shoots 54 to 102 at 2.80 and gives 11 Agility. The Hunter's ammunition adds the same
/// damage to both ends of a shot, so the ends are the row's and that addition.
#[test]
fn escape_plan_shoots_the_hotfixed_damage() {
    let bare = prepare(&hunter(&[(HUNTER_RANGED, 0)]));
    let with = prepare(&hunter(&[(HUNTER_RANGED, ESCAPE_PLAN)]));
    let ranged = &with["melee"]["ranged"];
    let min = ranged["base_damage_min"].as_f64().unwrap();
    let max = ranged["base_damage_max"].as_f64().unwrap();
    let ammunition = min - 54.0;
    assert!(ammunition >= 0.0, "{min}");
    assert_eq!(max, 102.0 + ammunition);
    assert_eq!(ranged["swing_speed"], 2.8);
    assert_primary(&with, &bare, "Agility", 11.0);
    let player = simulate(&with);
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0);
}

// ---------------------------------------------------------------------------------------------
// Filcher's Cowl.
// ---------------------------------------------------------------------------------------------

/// The cloak is 36 Armor, 11 Agility and 7 Spirit, with nothing registered for its stealth line,
/// and the Combat Rogue runs in it.
#[test]
fn filchers_cowl_is_a_plain_cloak() {
    let bare = prepare(&rogue(&[(BACK, 0)]));
    let with = prepare(&rogue(&[(BACK, FILCHERS_COWL)]));
    assert_primary(&with, &bare, "Agility", 11.0);
    assert_primary(&with, &bare, "Spirit", 7.0);
    // 36 from the cloak and 2 a point of Agility.
    let armor = gain(&with, &bare, "Armor") - 2.0 * gain(&with, &bare, "Agility");
    assert!((armor - 36.0).abs() < 1e-9, "{armor}");
    assert_eq!(effect_kinds(&with), effect_kinds(&bare));
    let player = simulate(&with);
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0);
}

// ---------------------------------------------------------------------------------------------
// Totem of Sustaining.
// ---------------------------------------------------------------------------------------------

/// Water Shield puts up three globes, and four with the totem worn. Nothing else changes.
#[test]
fn totem_of_sustaining_adds_a_water_shield_globe() {
    let bare = prepare(&restoration(&[(RANGED, 0)]));
    let with = prepare(&restoration(&[(RANGED, TOTEM_OF_SUSTAINING)]));
    assert_eq!(effect(&bare, "water_shield")["globes"], 3);
    assert_eq!(effect(&with, "water_shield")["globes"], 4);
    assert_eq!(aura(&bare, "Water Shield")["max_stacks"], 3);
    assert_eq!(aura(&with, "Water Shield")["max_stacks"], 4);
    assert_eq!(with["player"]["stats"], bare["player"]["stats"]);
}

/// The Restoration Shaman runs with the totem, through the gate.
#[test]
fn totem_of_sustaining_runs_on_a_shaman() {
    let with = prepare(&restoration(&[(RANGED, TOTEM_OF_SUSTAINING)]));
    let prepared: PreparedV2 = serde_json::from_value(with).unwrap();
    check_prepared(&prepared).unwrap_or_else(|err| panic!("{err:?}"));
    simulate_prepared(&prepared).unwrap();
}

/// A shaman without Water Shield wears the totem as a plain relic.
#[test]
fn totem_of_sustaining_is_plain_without_water_shield() {
    let bare = prepare(&elemental(&[(RANGED, 0)]));
    let with = prepare(&elemental(&[(RANGED, TOTEM_OF_SUSTAINING)]));
    assert_eq!(with["effects"], bare["effects"]);
    assert_eq!(with["player"]["stats"], bare["player"]["stats"]);
}

// ---------------------------------------------------------------------------------------------
// Weed Whacker.
// ---------------------------------------------------------------------------------------------

/// The axe prepares and runs in the Retribution Paladin's hands: 109 to 165 at 3.10 with 12
/// Strength, and its Grass Killer, 126 with a variance of 0.42 (100 to 152), lands at the
/// rateless chance on hit rate, 5.58% a swing.
#[test]
fn the_weed_whacker_is_an_axe_with_its_own_grass_killer() {
    let bare = prepare(&retribution(&[(MAIN_HAND, 0), (OFF_HAND, 0)]));
    let with = prepare(&retribution(&[(MAIN_HAND, WEED_WHACKER), (OFF_HAND, 0)]));
    let main_hand = &with["melee"]["main_hand"];
    assert_eq!(main_hand["base_damage_min"], 109.0);
    assert_eq!(main_hand["base_damage_max"], 165.0);
    assert_eq!(main_hand["swing_speed"], 3.1);
    assert_primary(&with, &bare, "Strength", 12.0);
    let proc = with["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| {
            effect["kind"] == "spell_data_damage_proc" && effect["trigger_aura"] == "Weed Whacker"
        })
        .expect("Weed Whacker's proc");
    assert_eq!(proc["average"], 126.0);
    assert_eq!(proc["variance"], 0.42);
    for chance in proc["chances"].as_array().unwrap() {
        let chance = chance["chance"].as_f64().unwrap();
        assert!((chance - 0.018 * 3.1).abs() < 1e-9, "{chance}");
    }
    let position = proc["spell"].as_u64().unwrap() as usize;
    assert_eq!(
        with["player"]["spells"][position]["action_id"]["spell_id"],
        WEED_WHACKER_GRASS_KILLER
    );
    let player = simulate(&with);
    assert!(total(&player, WEED_WHACKER_GRASS_KILLER, "damage") > 0.0);
}
