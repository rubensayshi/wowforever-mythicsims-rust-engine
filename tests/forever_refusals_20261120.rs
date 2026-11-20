//! Forever gear that refused real players' sims on 18 and 19 November 2026:
//!
//! - "no item with id 284050", three Hunter sims (a Quick Sim, a Best Gear run and a Droptimizer):
//!   High Elf's Recurve, a bow, item level 51, 41 to 77 at 2.50, +3 Agility.
//! - "no item with id 274484", two Holy Paladin sims: Axe of Inevitable Ruin, a two-hand axe, item
//!   level 42, 89 to 134 at 3.00.
//! - "no item with id 276632", two Arcane Mage Quick Sims: Wit's Edge, a Mage only main hand sword,
//!   item level 65, 39 to 74 at 2.00, +4 Intellect, damage and healing up to 74, 5 spell
//!   penetration.
//!
//! Each row is its Wowhead Forever tooltip of 20 November 2026 on the Item row of client
//! 1.60.1.70959 (wago.tools `product=wowf`; the ItemSparse rows are encrypted). The bow's and the
//! axe's chance on hit state no chance and no procs per minute, so they are left out, as Claymore
//! of Two Moons' proc is. Wit's Edge's 15% Clearcasting acts on melee attacks, which a Mage never
//! makes in a sim.
//!
//! - "an item swap changes /player/spells/4/flags, which the runtime does not follow", seven Best
//!   Gear runs of one Enhancement Shaman: the Manual Crowd Pummeler opener's swap set held
//!   Headmaster's Charge (13937), whose use is flagged as swapped from the pull and loses the flag
//!   as the staff goes on. The runtime clears it (the item use's swap callback); the coverage check
//!   allowed only the other way, a main set item's use gaining the flag.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[path = "support/paladin_113554.rs"]
mod paladin_113554;

const HIGH_ELFS_RECURVE: i64 = 284050;
const AXE_OF_INEVITABLE_RUIN: i64 = 274484;
const WITS_EDGE: i64 = 276632;
const HEADMASTERS_CHARGE: i64 = 13937;

const MAIN_HAND: usize = 14;

// The fixtures' own weapons: the Mage's staff, the Hunter's crossbow and the Paladin's two-hander.
const MAGE: (&str, i64) = (
    "fixtures/dev-notes-20261102/arcane-mage.request.json",
    20069,
);
const HUNTER: (&str, i64) = ("fixtures/forever-gaps/hunter-skysight.request.json", 18738);
const PALADIN: (&str, i64) = (
    "fixtures/forever-gaps/retribution-paladin-seal-of-wisdom.request.json",
    13982,
);
const SWAP: &str = "fixtures/forever-gaps/enhancement-shaman-swap-headmasters-charge.request.json";

fn request(fixture: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(20);
    // The Paladin fixture's talents read in push 113554's tree, which Forever's rules prepare.
    if fixture == PALADIN.0 {
        paladin_113554::forever_request(&mut request);
    }
    request
}

/// A fixture's request with the item `old` swapped for `new` (none takes it off). The fixtures'
/// equipment lists are compact, so an item is found by its id, not its slot.
fn replacing(fixture: &str, old: i64, new: Option<i64>) -> Value {
    let mut request = request(fixture);
    let player = &mut request["raid"]["parties"][0]["players"][0];
    let equipment = player["equipment"]["items"].as_array_mut().unwrap();
    let at = equipment
        .iter()
        .position(|item| item["id"] == old)
        .unwrap_or_else(|| panic!("{fixture} wears no {old}"));
    match new {
        Some(id) => equipment[at] = json!({ "id": id }),
        None => {
            equipment.remove(at);
        }
    }
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

fn stat(prepared: &Value, name: &str) -> f64 {
    prepared["player"]["stats"][name].as_f64().unwrap_or(0.0)
}

#[test]
fn high_elfs_recurve_is_the_hunters_bow() {
    let prepared = prepared(&replacing(HUNTER.0, HUNTER.1, Some(HIGH_ELFS_RECURVE)));
    simulate(&prepared);
    // The base damage carries the arrows' damage on both ends.
    let ranged = &prepared["melee"]["ranged"];
    let spread =
        ranged["base_damage_max"].as_f64().unwrap() - ranged["base_damage_min"].as_f64().unwrap();
    assert_eq!(spread, 77.0 - 41.0);
    assert_eq!(ranged["swing_speed"], 2.5);
}

#[test]
fn axe_of_inevitable_ruin_is_a_two_hander_without_a_proc() {
    let prepared = prepared(&replacing(
        PALADIN.0,
        PALADIN.1,
        Some(AXE_OF_INEVITABLE_RUIN),
    ));
    simulate(&prepared);
    let main_hand = &prepared["melee"]["main_hand"];
    assert_eq!(main_hand["base_damage_min"], 89.0);
    assert_eq!(main_hand["base_damage_max"], 134.0);
    assert_eq!(main_hand["swing_speed"], 3.0);
    let labels: Vec<&str> = prepared["player"]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|aura| aura["label"].as_str())
        .collect();
    assert!(labels.iter().all(|label| !label.contains("Inevitable")));
}

#[test]
fn wits_edge_gives_the_mage_its_tooltip_stats() {
    let without = prepared(&replacing(MAGE.0, MAGE.1, None));
    let with = prepared(&replacing(MAGE.0, MAGE.1, Some(WITS_EDGE)));
    simulate(&with);
    assert_eq!(
        stat(&with, "SpellDamage") - stat(&without, "SpellDamage"),
        74.0
    );
    assert_eq!(
        stat(&with, "HealingPower") - stat(&without, "HealingPower"),
        74.0
    );
    assert_eq!(
        stat(&with, "SpellPiercing") - stat(&without, "SpellPiercing"),
        5.0
    );
    // Intellect passes through the Mage's percent bonuses.
    assert!((4.0..=5.0).contains(&(stat(&with, "Intellect") - stat(&without, "Intellect"))));
}

/// The refused Best Gear set: the Pummeler opener swaps to Headmaster's Charge, whose use is
/// pressable only once the staff is on.
#[test]
fn a_swap_set_item_use_passes_the_gate_and_follows_the_swap() {
    let request = request(SWAP);
    let player = &request["raid"]["parties"][0]["players"][0];
    assert_eq!(
        player["itemSwap"]["items"][MAIN_HAND]["id"],
        HEADMASTERS_CHARGE
    );
    let prepared = prepared(&request);
    let spells = prepared["player"]["spells"].as_array().unwrap();
    let staff = spells
        .iter()
        .find(|spell| spell["action_id"]["item_id"] == HEADMASTERS_CHARGE)
        .expect("the staff's use is a spell");
    // Off at the pull: the Pummeler is in the hand.
    assert!(staff["flags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|flag| flag == "SpellFlagSwapped"));
    simulate(&prepared);
}
