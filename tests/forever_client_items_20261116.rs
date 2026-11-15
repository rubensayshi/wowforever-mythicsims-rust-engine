//! Forever client items and a set bonus that refused real players' sims on 14 and 15 November
//! 2026, from Wowhead's Forever tooltips and the client rows wago.tools serves (`product=wowf`,
//! `build=1.60.1.70938`) on 16 November 2026:
//!
//! - Bolt's Charged Eyeball (280032), trinket, item level 63, Unique-Equipped: "Equip: Dealing
//!   damage has a chance to deal 60 Nature damage to you or your current target. Deals 3 times as
//!   much damage to Dinosaurs. (Proc chance: 15%, 3s cooldown)". ItemEffect 236825 is an equip
//!   (trigger type 1) of Grounded Electroshocks (1311505): SpellAuraOptions 248082, ProcChance
//!   15, ProcCategoryRecovery 3000, ProcTypeMask 332116 (melee, ranged and spell damage dealt,
//!   periodic damage included); SpellEffect 1341855, a dummy aura triggering Grounded
//!   Electroshock (1315996), a School Damage of 60, Nature, magic defense type, on implicit
//!   target 25 (SpellEffect 1348627, SpellMisc 862172, SpellCategories 188422). No row holds a
//!   hit on the wearer. Sixteen refusals.
//! - Ward of the Dead (217727), trinket, item level 1, Unique: "Equip: Reveals the grave which
//!   hides an offering." No stats and no fight effect. Six refusals.
//! - Conqueror's Pendant (270126), neck, item level 58: +7 Stamina, +7 Intellect, +5 Spirit,
//!   "Equip: Increases damage and healing done by magical spells and effects by up to 13." One
//!   refusal.
//! - Brush Cutter (274265), one-hand axe, item level 57, 66 to 124, speed 2.80, +5 Stamina:
//!   "Chance on hit: Assault the target with a dangerous toxin, dealing 50 to 76 Nature damage.
//!   Deals 3 times as much damage to Plants." ItemEffect 233726 is a chance on hit of Grass Killer
//!   (1293518), as Deadhead Blade's is. Three refusals.
//! - Magister's Regalia (5), on a spec that is not a healer: "Your spellcasts have a 5% chance to
//!   energize you for 200 mana." (ItemSetSpell 7906, spell 450527 casting Sudden Insight 450554).
//!   Two refusals, both Fire Mage.
//!
//! No refused request is copied here: the tests dress fixture characters, the Combat Rogue of
//! `fixtures/forever-gaps/combat-rogue-self-casts-at-player`, the Demonology Warlock of
//! `fixtures/forever-gaps/demonology-warlock-black-book` and the Retribution Paladin of
//! `fixtures/paladin/retribution-paladin-unsealed-wrath-command-echoes`.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared,
    PrepareError, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[path = "support/rogue_113483.rs"]
mod rogue_113483;

const BOLTS_CHARGED_EYEBALL: i64 = 280032;
const GROUNDED_ELECTROSHOCK: i64 = 1315996;
const WARD_OF_THE_DEAD: i64 = 217727;
const CONQUERORS_PENDANT: i64 = 270126;
const BRUSH_CUTTER: i64 = 274265;
const GRASS_KILLER: i64 = 1293518;
/// Five pieces of Magister's Regalia: crown, mantle, robes, belt and leggings.
const MAGISTER_PIECES: [(usize, i64); 5] =
    [(0, 16686), (2, 16689), (4, 16688), (7, 16685), (8, 16687)];

const NECK: usize = 1;
const TRINKET: usize = 13;
const MAIN_HAND: usize = 14;
const OFF_HAND: usize = 15;

fn player(request: &mut Value) -> &mut Value {
    &mut request["raid"]["parties"][0]["players"][0]
}

/// A fixture's request (a path under `fixtures/`) with these items in these slots and no
/// consumables.
fn wearing(fixture: &str, items: &[(usize, Value)]) -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("fixtures/{fixture}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(20);
    let player = player(&mut request);
    player["consumables"] = json!({});
    let equipment = player["equipment"]["items"].as_array_mut().unwrap();
    equipment.resize(17, json!({}));
    for (slot, item) in items {
        equipment[*slot] = item.clone();
    }
    request
}

/// The Combat Rogue. The fixture's talents are read in the trees Forever's rules prepare.
fn rogue(items: &[(usize, Value)]) -> Value {
    let mut request = wearing("forever-gaps/combat-rogue-self-casts-at-player", items);
    rogue_113483::forever_request(&mut request);
    request
}

/// The Demonology Warlock, whose rotation keeps damage over time spells up.
fn warlock(items: &[(usize, Value)]) -> Value {
    wearing("forever-gaps/demonology-warlock-black-book", items)
}

/// The Retribution Paladin, without the fixture's item swap.
fn paladin(items: &[(usize, Value)]) -> Value {
    let mut request = wearing(
        "paladin/retribution-paladin-unsealed-wrath-command-echoes",
        items,
    );
    let player = player(&mut request).as_object_mut().unwrap();
    player.remove("itemSwap");
    player.insert("enableItemSwap".to_string(), json!(false));
    request
}

fn prepared(request: &Value) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever)
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

fn refusal(request: &Value, rules: Rules) -> String {
    match prepare_json_with(request.to_string().as_bytes(), "rust", rules) {
        Err(PrepareError::Refused(refusal)) => refusal.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
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

fn stat(prepared: &Value, name: &str) -> f64 {
    prepared["player"]["stats"][name].as_f64().unwrap_or(0.0)
}

fn proc<'a>(prepared: &'a Value, item: &str) -> &'a Value {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "spell_data_damage_proc" && effect["trigger_aura"] == item)
        .unwrap_or_else(|| panic!("{item}'s proc"))
}

/// The spell a proc casts, by its spellbook position.
fn proc_spell<'a>(prepared: &'a Value, item: &str) -> &'a Value {
    let position = proc(prepared, item)["spell"].as_u64().unwrap() as usize;
    &prepared["player"]["spells"][position]
}

fn aura<'a>(prepared: &'a Value, label: &str) -> &'a Value {
    prepared["player"]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|aura| aura["label"] == label)
        .unwrap_or_else(|| panic!("aura {label}"))
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

/// A request against targets of this creature type.
fn against(mut request: Value, mob_type: &str) -> Value {
    for target in request["encounter"]["targets"].as_array_mut().unwrap() {
        target["mobType"] = json!(mob_type);
    }
    request
}

// ---------------------------------------------------------------------------------------------
// Bolt's Charged Eyeball.
// ---------------------------------------------------------------------------------------------

/// The eyeball prepares, passes the gate and runs on the Combat Rogue, and the shock lands.
#[test]
fn the_eyeball_prepares_and_runs_on_a_rogue() {
    let player = simulate(&prepared(&rogue(&[(
        TRINKET,
        json!({ "id": BOLTS_CHARGED_EYEBALL }),
    )])));
    assert!(total(&player, GROUNDED_ELECTROSHOCK, "damage") > 0.0);
    assert!(total(&player, GROUNDED_ELECTROSHOCK, "hits") > 0.0);
}

/// Grounded Electroshock is the client row's: a Nature hit of 60 on the magic table, with no
/// variance, no damage over time and no coefficient, at the row's 15% and 3 sec recovery. The
/// mask's periodic damage is heard (`ticks`).
#[test]
fn the_shock_is_a_nature_hit_at_fifteen_percent_every_three_seconds_at_most() {
    let with = prepared(&rogue(&[(TRINKET, json!({ "id": BOLTS_CHARGED_EYEBALL }))]));
    let proc = proc(&with, "Bolt's Charged Eyeball");
    assert_eq!(proc["proc_chance"], 0.15);
    assert_eq!(proc["average"], 60.0);
    assert_eq!(proc["variance"], 0.0);
    assert_eq!(proc["landed_only"], true);
    assert_eq!(proc["ticks"], true);
    assert!(proc.get("periodic").is_none());
    assert!(proc.get("area").is_none());
    assert!(proc.get("struck").is_none());
    assert!(!proc["trigger_spells"].as_array().unwrap().is_empty());
    let aura = aura(&with, "Bolt's Charged Eyeball");
    assert_eq!(aura["icd"]["duration_ns"], 3_000_000_000_i64);
    assert!(aura["callbacks"]
        .as_array()
        .unwrap()
        .contains(&json!("on_periodic_damage_dealt")));
    let spell = proc_spell(&with, "Bolt's Charged Eyeball");
    assert_eq!(spell["action_id"]["spell_id"], GROUNDED_ELECTROSHOCK);
    assert_eq!(spell["school"], 8);
    assert_eq!(spell["defense_type"], "DefenseTypeMagic");
    assert_eq!(spell["bonus_coefficient"], 0.0);
    assert_eq!(spell["damage_multiplier"], 1.0);
}

/// The Warlock's damage over time ticks are heard as well: a Warlock whose rotation keeps
/// Corruption and Immolate up shocks more often than one whose damage over time spells are out of
/// the listener's spells.
#[test]
fn the_eyeball_hears_a_warlocks_ticks() {
    let with = prepared(&warlock(&[(
        TRINKET,
        json!({ "id": BOLTS_CHARGED_EYEBALL }),
    )]));
    assert_eq!(proc(&with, "Bolt's Charged Eyeball")["ticks"], true);
    let player = simulate(&with);
    let shocks = total(&player, GROUNDED_ELECTROSHOCK, "hits");
    assert!(shocks > 0.0);

    let mut hits_only = with.clone();
    let effect = hits_only["effects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|effect| effect["trigger_aura"] == "Bolt's Charged Eyeball")
        .unwrap();
    effect.as_object_mut().unwrap().remove("ticks");
    let without_ticks = total(&simulate(&hits_only), GROUNDED_ELECTROSHOCK, "hits");
    assert!(shocks > without_ticks, "{shocks} {without_ticks}");
}

/// "Deals 3 times as much damage to Dinosaurs": no creature type a request names is one, so the
/// hit is the row's own against every one of them.
#[test]
fn the_shock_is_never_tripled() {
    let plain = prepared(&rogue(&[(TRINKET, json!({ "id": BOLTS_CHARGED_EYEBALL }))]));
    for mob_type in [
        "MobTypeBeast",
        "MobTypeDemon",
        "MobTypeDragonkin",
        "MobTypeElemental",
        "MobTypeGiant",
        "MobTypeHumanoid",
        "MobTypeMechanical",
        "MobTypeUndead",
    ] {
        let versus = prepared(&against(
            rogue(&[(TRINKET, json!({ "id": BOLTS_CHARGED_EYEBALL }))]),
            mob_type,
        ));
        assert_eq!(
            proc_spell(&versus, "Bolt's Charged Eyeball")["damage_multiplier"],
            1.0,
            "{mob_type}"
        );
        assert_eq!(
            proc(&versus, "Bolt's Charged Eyeball"),
            proc(&plain, "Bolt's Charged Eyeball"),
            "{mob_type}"
        );
    }
}

/// The trinket has no stats: the same character with the slot empty has the same.
#[test]
fn the_eyeball_has_no_stats() {
    let with = prepared(&rogue(&[(TRINKET, json!({ "id": BOLTS_CHARGED_EYEBALL }))]));
    let without = prepared(&rogue(&[(TRINKET, json!({}))]));
    assert_eq!(with["player"]["stats"], without["player"]["stats"]);
}

// ---------------------------------------------------------------------------------------------
// Ward of the Dead and Conqueror's Pendant.
// ---------------------------------------------------------------------------------------------

/// The ward is a trinket with nothing on it: the same stats and effects as the empty slot.
#[test]
fn the_ward_is_an_item_with_no_stats() {
    let with = prepared(&rogue(&[(TRINKET, json!({ "id": WARD_OF_THE_DEAD }))]));
    let without = prepared(&rogue(&[(TRINKET, json!({}))]));
    simulate(&with);
    assert_eq!(with["player"]["stats"], without["player"]["stats"]);
    assert_eq!(effect_kinds(&with), effect_kinds(&without));
}

/// The pendant is its tooltip's stats on the Warlock: 7 Stamina, 7 Intellect, 5 Spirit and 13
/// spell damage and healing over the empty slot, before the character's own multipliers.
#[test]
fn the_pendant_is_its_tooltips_stats() {
    let with = prepared(&warlock(&[(NECK, json!({ "id": CONQUERORS_PENDANT }))]));
    let without = prepared(&warlock(&[(NECK, json!({}))]));
    simulate(&with);
    assert!(stat(&with, "Stamina") - stat(&without, "Stamina") >= 7.0);
    assert!(stat(&with, "Intellect") - stat(&without, "Intellect") >= 7.0);
    assert!(stat(&with, "Spirit") - stat(&without, "Spirit") >= 5.0);
    assert_eq!(
        stat(&with, "SpellDamage") - stat(&without, "SpellDamage"),
        13.0
    );
    assert_eq!(
        stat(&with, "HealingPower") - stat(&without, "HealingPower"),
        13.0
    );
    assert_eq!(effect_kinds(&with), effect_kinds(&without));
}

// ---------------------------------------------------------------------------------------------
// Brush Cutter.
// ---------------------------------------------------------------------------------------------

/// The axe prepares and runs in the Retribution Paladin's main hand: 66 to 124 at 2.80, and its
/// Grass Killer lands at the rateless chance on hit rate, 5.04% a swing.
#[test]
fn the_brush_cutter_is_an_axe_with_grass_killer() {
    let with = prepared(&paladin(&[
        (MAIN_HAND, json!({ "id": BRUSH_CUTTER })),
        (OFF_HAND, json!({})),
    ]));
    let main_hand = &with["melee"]["main_hand"];
    assert_eq!(main_hand["base_damage_min"], 66.0);
    assert_eq!(main_hand["base_damage_max"], 124.0);
    assert_eq!(main_hand["swing_speed"], 2.8);
    let proc = proc(&with, "Brush Cutter");
    assert_eq!(proc["average"], 63.0);
    assert_eq!(proc["variance"], 0.42);
    for chance in proc["chances"].as_array().unwrap() {
        let chance = chance["chance"].as_f64().unwrap();
        assert!((chance - 0.018 * 2.8).abs() < 1e-9, "{chance}");
    }
    assert_eq!(
        proc_spell(&with, "Brush Cutter")["action_id"]["spell_id"],
        GRASS_KILLER
    );
    let player = simulate(&with);
    assert!(total(&player, GRASS_KILLER, "damage") > 0.0);
}

// ---------------------------------------------------------------------------------------------
// Magister's Regalia (5) on a damage spec.
// ---------------------------------------------------------------------------------------------

/// Five pieces on the Warlock: Sudden Insight is described as it is for a healer, 200 mana on 5%
/// of the spell casts, and the sim runs.
#[test]
fn magisters_regalia_five_restores_mana_on_a_damage_spec() {
    let five: Vec<(usize, Value)> = MAGISTER_PIECES
        .iter()
        .map(|(slot, id)| (*slot, json!({ "id": id })))
        .collect();
    let with = prepared(&warlock(&five));
    let insight = with["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| {
            effect["kind"] == "mana_on_cast" && effect["trigger_aura"] == "Magister's Regalia 5P"
        })
        .expect("Sudden Insight");
    assert_eq!(insight["proc_chance"], 0.05);
    let spells = insight["spells"].as_array().unwrap();
    assert!(!spells.is_empty());
    assert!(spells.iter().all(|spell| spell["mana"] == 200.0));
    simulate(&with);

    // Four pieces describe nothing.
    let four = prepared(&warlock(&five[..4]));
    assert!(!effect_kinds(&four).contains(&json!("mana_on_cast")));
}

// ---------------------------------------------------------------------------------------------
// The reference's rules.
// ---------------------------------------------------------------------------------------------

/// The reference's rules have no row for any of the four items and refuse each as unknown.
#[test]
fn the_reference_rules_still_refuse_the_items() {
    for (slot, id) in [
        (TRINKET, BOLTS_CHARGED_EYEBALL),
        (TRINKET, WARD_OF_THE_DEAD),
        (NECK, CONQUERORS_PENDANT),
        (MAIN_HAND, BRUSH_CUTTER),
    ] {
        let reason = refusal(
            &paladin(&[(slot, json!({ "id": id })), (OFF_HAND, json!({}))]),
            Rules::GoReference,
        );
        assert!(
            reason.contains(&format!("no item with id {id}")),
            "{reason}"
        );
    }
}
