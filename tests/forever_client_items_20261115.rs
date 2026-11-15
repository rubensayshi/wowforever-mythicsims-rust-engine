//! Forever client items that refused real players' sims on 13 and 14 November 2026 ("no item with
//! id N"), from Wowhead's Forever tooltips and the client rows wago.tools serves (`product=wowf`,
//! `build=1.60.1.70938`) on 15 November 2026:
//!
//! - Behemoth Bones (280065), trinket, item level 61, Unique-Equipped: "Equip: +34 Attack
//!   Power." and "Use: Crack open the bone, healing yourself for 480 to 530 and your nearest ally
//!   for 480 to 530. (5 Min Cooldown)". ItemEffect 237440 is a use (trigger type 0) of Behemoth
//!   Bone (1315849): a Heal (10) of 505 with variance 0.1 on the wearer and a second one on the
//!   nearest ally (1315854). The use is never pressed, as Zarla's Snack Pouch's is not. Fourteen
//!   refusals.
//! - Jom-jom's Gabbar (274952), one-hand dagger, item level 42, 32 to 60, speed 1.90: "Chance on
//!   hit: Launch a boulder, dealing 98 Physical damage." ItemEffect 234122 is a chance on hit
//!   (trigger type 2) of Bolder'ok Boulder (1295736) with no rate and no cooldown: a School Damage
//!   of 98, Physical, on the ranged table (SpellEffect 1317973, SpellMisc 843131, SpellCategories
//!   184301). Two Combat Rogue refusals.
//! - Deadhead Blade (274271), one-hand dagger, item level 57, 42 to 80, speed 1.80, +5 Agility:
//!   "Chance on hit: Assault the target with a dangerous toxin, dealing 50 to 76 Nature damage.
//!   Deals 3 times as much damage to Plants." ItemEffect 233730 is a chance on hit of Grass Killer
//!   (1293518) with no rate and no cooldown: a School Damage of 63 with variance 0.42, Nature, on
//!   the magic table (SpellEffect 1314222, SpellMisc 840948, SpellCategories 183662), and a Dummy
//!   of 3 (SpellEffect 1314223), the tripling against Plants, which no creature type of the
//!   client or of a request names. One Survival Hunter refusal.
//! - Claymore of Two Moons (280080), two-hand sword, item level 57, 121 to 182, speed 3.10, +17
//!   Stamina: "Equip: Your attacks have a chance to Wax, dealing 189 Arcane damage or Wane,
//!   granting 40 Strength for 10 sec." ItemEffect 236622 is an equip of Claymore of Two Moons
//!   (1309550), a dummy aura with ProcChance 100 on melee attacks and no trigger spell: no row
//!   states the chance or the split between Waxing Moon (1319378) and Waning Moon (1319382), so
//!   the proc is left out. One Droptimizer refusal.
//! - Savage Gladiator Shoulders (277117), mail shoulders, item level 57: 254 Armor, +11 Strength,
//!   +16 Stamina, "Equip: Improves your chance to hit by 1.0%." A sixth piece of The Gladiator
//!   (ItemSet 1), whose five Classic pieces are reference rows and whose bonuses nothing
//!   registers. One Warrior refusal.
//!
//! No refused request was at hand, so the tests dress fixture characters: the Combat Rogue of
//! `fixtures/forever-gaps/combat-rogue-self-casts-at-player` and the Retribution Paladin of
//! `fixtures/paladin/retribution-paladin-unsealed-wrath-command-echoes`.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared,
    PrepareError, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[path = "support/rogue_113483.rs"]
mod rogue_113483;

const BEHEMOTH_BONES: i64 = 280065;
const JOM_JOMS_GABBAR: i64 = 274952;
const BOLDEROK_BOULDER: i64 = 1295736;
const DEADHEAD_BLADE: i64 = 274271;
const GRASS_KILLER: i64 = 1293518;
const CLAYMORE_OF_TWO_MOONS: i64 = 280080;
const SAVAGE_GLADIATOR_SHOULDERS: i64 = 277117;
/// The five Classic pieces of The Gladiator: helm, chest, hands, legs and feet.
const GLADIATOR_PIECES: [(usize, i64); 5] =
    [(0, 11729), (4, 11726), (6, 11730), (8, 11728), (9, 11731)];

const SHOULDER: usize = 2;
const TRINKET: usize = 13;
const MAIN_HAND: usize = 14;
const OFF_HAND: usize = 15;

fn player(request: &mut Value) -> &mut Value {
    &mut request["raid"]["parties"][0]["players"][0]
}

/// A fixture's request (a path under `fixtures/`) with these items in these slots and no
/// consumables, whose sharpening stone would add weapon damage.
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

/// The Retribution Paladin, who wields a two-hand weapon and wears mail, without the fixture's
/// item swap, which would take the weapon out of the hand.
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
        .unwrap_or_else(|| panic!("{item}'s chance on hit"))
}

/// The spell a chance on hit casts, by its spellbook position.
fn proc_spell<'a>(prepared: &'a Value, item: &str) -> &'a Value {
    let position = proc(prepared, item)["spell"].as_u64().unwrap() as usize;
    &prepared["player"]["spells"][position]
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
// Behemoth Bones.
// ---------------------------------------------------------------------------------------------

/// The bones prepare and run on the Combat Rogue: 34 attack power, melee and ranged, over the
/// same character with the slot empty, and no other stat.
#[test]
fn the_bones_are_thirty_four_attack_power() {
    let with = prepared(&rogue(&[(TRINKET, json!({ "id": BEHEMOTH_BONES }))]));
    let without = prepared(&rogue(&[(TRINKET, json!({}))]));
    simulate(&with);
    for name in ["AttackPower", "RangedAttackPower"] {
        assert_eq!(stat(&with, name) - stat(&without, name), 34.0, "{name}");
    }
    for name in [
        "Strength",
        "Agility",
        "Stamina",
        "Armor",
        "PhysicalCritPercent",
    ] {
        assert_eq!(stat(&with, name), stat(&without, name), "{name}");
    }
}

/// The heal is never pressed: the bones register no effect and no cooldown, so the prepared input
/// holds the same effects and cooldowns as the empty slot's.
#[test]
fn the_bones_heal_is_not_registered() {
    let with = prepared(&rogue(&[(TRINKET, json!({ "id": BEHEMOTH_BONES }))]));
    let without = prepared(&rogue(&[(TRINKET, json!({}))]));
    assert_eq!(effect_kinds(&with), effect_kinds(&without));
    assert_eq!(
        with["player"]["major_cooldowns"],
        without["player"]["major_cooldowns"]
    );
    assert!(!with.to_string().contains("1315849"));
}

// ---------------------------------------------------------------------------------------------
// Jom-jom's Gabbar.
// ---------------------------------------------------------------------------------------------

/// The dagger prepares, passes the gate and runs in the Combat Rogue's main hand, and the boulder
/// lands.
#[test]
fn the_gabbar_prepares_and_runs() {
    let player = simulate(&prepared(&rogue(&[(
        MAIN_HAND,
        json!({ "id": JOM_JOMS_GABBAR }),
    )])));
    assert!(total(&player, BOLDEROK_BOULDER, "damage") > 0.0);
    assert!(total(&player, BOLDEROK_BOULDER, "hits") > 0.0);
}

/// The row is the tooltip's dagger: 32 to 60 at 1.90, with no stats.
#[test]
fn the_gabbar_row_is_the_tooltips_dagger() {
    let with = prepared(&rogue(&[(MAIN_HAND, json!({ "id": JOM_JOMS_GABBAR }))]));
    let main_hand = &with["melee"]["main_hand"];
    assert_eq!(main_hand["base_damage_min"], 32.0);
    assert_eq!(main_hand["base_damage_max"], 60.0);
    assert_eq!(main_hand["swing_speed"], 1.9);
}

/// Bolder'ok Boulder is the client row's: a Physical hit of 98 on the ranged table, with no
/// variance, no damage over time and no coefficient. No rate is stated: 3.42% a swing at 1.90.
#[test]
fn the_boulder_is_a_physical_hit_on_the_weapons_proc_rate() {
    let with = prepared(&rogue(&[(MAIN_HAND, json!({ "id": JOM_JOMS_GABBAR }))]));
    let proc = proc(&with, "Jom-jom's Gabbar");
    assert_eq!(proc["landed_only"], true);
    assert_eq!(proc["average"], 98.0);
    assert_eq!(proc["variance"], 0.0);
    assert!(proc.get("periodic").is_none());
    assert!(proc.get("area").is_none());
    let chances = proc["chances"].as_array().unwrap();
    assert!(!chances.is_empty());
    for chance in chances {
        let chance = chance["chance"].as_f64().unwrap();
        assert!((chance - 0.018 * 1.9).abs() < 1e-9, "{chance}");
    }
    let spell = proc_spell(&with, "Jom-jom's Gabbar");
    assert_eq!(spell["action_id"]["spell_id"], BOLDEROK_BOULDER);
    assert_eq!(spell["school"], 1);
    assert_eq!(spell["defense_type"], "DefenseTypeRanged");
    assert_eq!(spell["bonus_coefficient"], 0.0);
    assert_eq!(spell["damage_multiplier"], 1.0);
}

// ---------------------------------------------------------------------------------------------
// Deadhead Blade.
// ---------------------------------------------------------------------------------------------

/// The blade prepares, passes the gate and runs in the Combat Rogue's main hand, and Grass Killer
/// lands.
#[test]
fn the_blade_prepares_and_runs() {
    let player = simulate(&prepared(&rogue(&[(
        MAIN_HAND,
        json!({ "id": DEADHEAD_BLADE }),
    )])));
    assert!(total(&player, GRASS_KILLER, "damage") > 0.0);
    assert!(total(&player, GRASS_KILLER, "hits") > 0.0);
}

/// The row is the tooltip's dagger: 42 to 80 at 1.80, with 5 Agility.
#[test]
fn the_blade_row_is_the_tooltips_dagger() {
    let with = prepared(&rogue(&[(MAIN_HAND, json!({ "id": DEADHEAD_BLADE }))]));
    let other = prepared(&rogue(&[(MAIN_HAND, json!({ "id": JOM_JOMS_GABBAR }))]));
    let main_hand = &with["melee"]["main_hand"];
    assert_eq!(main_hand["base_damage_min"], 42.0);
    assert_eq!(main_hand["base_damage_max"], 80.0);
    assert_eq!(main_hand["swing_speed"], 1.8);
    // Jom-jom's Gabbar has no stats, so the difference is the blade's Agility before the
    // character's own multipliers.
    assert!(stat(&with, "Agility") - stat(&other, "Agility") >= 5.0);
}

/// Grass Killer is the client row's: a Nature hit of 63 with a variance of 0.42 (50 to 76) on the
/// magic table, with no damage over time and no coefficient. No rate is stated: 3.24% a swing at
/// 1.80.
#[test]
fn grass_killer_is_a_nature_hit_on_the_weapons_proc_rate() {
    let with = prepared(&rogue(&[(MAIN_HAND, json!({ "id": DEADHEAD_BLADE }))]));
    let proc = proc(&with, "Deadhead Blade");
    assert_eq!(proc["landed_only"], true);
    assert_eq!(proc["average"], 63.0);
    assert_eq!(proc["variance"], 0.42);
    assert!(proc.get("periodic").is_none());
    assert!(proc.get("area").is_none());
    for chance in proc["chances"].as_array().unwrap() {
        let chance = chance["chance"].as_f64().unwrap();
        assert!((chance - 0.018 * 1.8).abs() < 1e-9, "{chance}");
    }
    let spell = proc_spell(&with, "Deadhead Blade");
    assert_eq!(spell["action_id"]["spell_id"], GRASS_KILLER);
    assert_eq!(spell["school"], 8);
    assert_eq!(spell["defense_type"], "DefenseTypeMagic");
    assert_eq!(spell["bonus_coefficient"], 0.0);
    assert_eq!(spell["damage_multiplier"], 1.0);
}

/// "Deals 3 times as much damage to Plants": no creature type a request names is a Plant, so the
/// hit is the row's own against every one of them.
#[test]
fn grass_killer_is_never_tripled() {
    let plain = prepared(&rogue(&[(MAIN_HAND, json!({ "id": DEADHEAD_BLADE }))]));
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
            rogue(&[(MAIN_HAND, json!({ "id": DEADHEAD_BLADE }))]),
            mob_type,
        ));
        assert_eq!(
            proc_spell(&versus, "Deadhead Blade")["damage_multiplier"],
            1.0,
            "{mob_type}"
        );
        assert_eq!(
            proc(&versus, "Deadhead Blade"),
            proc(&plain, "Deadhead Blade"),
            "{mob_type}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Claymore of Two Moons.
// ---------------------------------------------------------------------------------------------

/// The claymore prepares, passes the gate and runs in the Retribution Paladin's hands. The row is
/// the tooltip's sword: 121 to 182 at 3.10.
#[test]
fn the_claymore_row_is_the_tooltips_two_hand_sword() {
    let with = prepared(&paladin(&[
        (MAIN_HAND, json!({ "id": CLAYMORE_OF_TWO_MOONS })),
        (OFF_HAND, json!({})),
    ]));
    simulate(&with);
    let main_hand = &with["melee"]["main_hand"];
    assert_eq!(main_hand["base_damage_min"], 121.0);
    assert_eq!(main_hand["base_damage_max"], 182.0);
    assert_eq!(main_hand["swing_speed"], 3.1);
}

/// Wax and Wane are left out: the claymore registers no effect, so the prepared input holds the
/// effects of another plain two-hand sword's and names none of the three spells.
#[test]
fn the_claymores_proc_is_left_out() {
    let with = prepared(&paladin(&[
        (MAIN_HAND, json!({ "id": CLAYMORE_OF_TWO_MOONS })),
        (OFF_HAND, json!({})),
    ]));
    assert!(with["effects"]
        .as_array()
        .unwrap()
        .iter()
        .all(|effect| effect["trigger_aura"] != "Claymore of Two Moons"));
    let text = with.to_string();
    for spell in ["1309550", "1319378", "1319382"] {
        assert!(!text.contains(spell), "{spell}");
    }
    let player = simulate(&with);
    assert_eq!(total(&player, 1319378, "damage"), 0.0);
}

// ---------------------------------------------------------------------------------------------
// Savage Gladiator Shoulders.
// ---------------------------------------------------------------------------------------------

/// The shoulders prepare and run on the Retribution Paladin: 254 Armor, 11 Strength, 16 Stamina
/// and 10 hit rating over the same character with the slot empty, before the character's own
/// multipliers.
#[test]
fn the_shoulders_row_is_the_tooltips_mail() {
    let with = prepared(&paladin(&[(
        SHOULDER,
        json!({ "id": SAVAGE_GLADIATOR_SHOULDERS }),
    )]));
    let without = prepared(&paladin(&[(SHOULDER, json!({}))]));
    simulate(&with);
    assert!(stat(&with, "Armor") - stat(&without, "Armor") >= 254.0);
    assert!(stat(&with, "Strength") - stat(&without, "Strength") >= 11.0);
    assert!(stat(&with, "Stamina") - stat(&without, "Stamina") >= 16.0);
}

/// The Gladiator's bonuses are registered for no piece: with the five Classic pieces and the
/// shoulders on, the prepared input names no aura of the set and holds the effects of the same
/// character in the five pieces alone.
#[test]
fn the_gladiator_set_stays_unregistered_with_the_sixth_piece() {
    let five: Vec<(usize, Value)> = GLADIATOR_PIECES
        .iter()
        .map(|(slot, id)| (*slot, json!({ "id": id })))
        .collect();
    let mut six = five.clone();
    six.push((SHOULDER, json!({ "id": SAVAGE_GLADIATOR_SHOULDERS })));
    let mut bare = five.clone();
    bare.push((SHOULDER, json!({})));
    let with = prepared(&paladin(&six));
    let without = prepared(&paladin(&bare));
    simulate(&with);
    assert!(!with.to_string().contains("The Gladiator"));
    assert_eq!(effect_kinds(&with), effect_kinds(&without));
}

// ---------------------------------------------------------------------------------------------
// The reference's rules.
// ---------------------------------------------------------------------------------------------

/// The reference's rules have no row for any of the five and refuse each as unknown.
#[test]
fn the_reference_rules_still_refuse_the_items() {
    for (slot, id) in [
        (TRINKET, BEHEMOTH_BONES),
        (MAIN_HAND, JOM_JOMS_GABBAR),
        (MAIN_HAND, DEADHEAD_BLADE),
        (MAIN_HAND, CLAYMORE_OF_TWO_MOONS),
        (SHOULDER, SAVAGE_GLADIATOR_SHOULDERS),
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
