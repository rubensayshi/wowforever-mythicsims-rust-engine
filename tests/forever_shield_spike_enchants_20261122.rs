//! The shield spike enchants under Forever's rules, reported on 22 November 2026: a Mithril
//! Shield Spike on a tank's shield dealt no damage in the sim. The live client's rows (wago.tools
//! `product=wowf`) give each SpellItemEnchantment one equip spell (Effect 3) of the same shape as
//! the Girdle of Extracted Answers' Shield Spike: a proc trigger damage aura on melee hits taken,
//! a ProcChance of 100 and a 5 sec spell cooldown.
//!
//! - Iron Shield Spike (43, "Iron Spike (8-12)"): 9784, 10 with a variance of 0.4.
//! - Mithril Shield Spike (463, "Mithril Spike (16-20)"): 9782, 18 with a variance of 0.2222.
//! - Thorium Shield Spike (1704, "Thorium Spike (20-30)"): 16624, 25 with a variance of 0.4.
//!
//! The reference's rules give the enchants no effect, and still do.

use forever_engine::{
    check_prepared, contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

#[path = "support/warrior_trees_20261104.rs"]
mod warrior_trees_20261104;

const IRON_SHIELD_SPIKE: i64 = 43;
const MITHRIL_SHIELD_SPIKE: i64 = 463;
const THORIUM_SHIELD_SPIKE: i64 = 1704;

const IRON_SPIKE_SPELL: i64 = 9784;
const MITHRIL_SPIKE_SPELL: i64 = 9782;
const THORIUM_SPIKE_SPELL: i64 = 16624;

fn read(path: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

/// The equipment entry of the shield, found by its id: the request's lists are compact and
/// preparation places each item by type.
fn enchant_shield(request: &mut Value, shield: i64, enchant: i64) {
    request["simOptions"]["iterations"] = json!(50);
    request["simOptions"]["debugFirstIteration"] = json!(false);
    let items = request["raid"]["parties"][0]["players"][0]["equipment"]["items"]
        .as_array_mut()
        .unwrap();
    let entry = items
        .iter_mut()
        .find(|item| item["id"].as_i64() == Some(shield))
        .unwrap_or_else(|| panic!("no shield {shield}"));
    entry["enchant"] = json!(enchant);
}

/// The production Protection Warrior, who tanks the target with Draconian Deflector (18756).
fn tank_warrior(enchant: i64) -> Value {
    let mut request = read(
        "fixtures/mage/prepared-v2/protection-warrior-blood-craze-ephemeral-power.request.json",
    );
    warrior_trees_20261104::forever_request(&mut request);
    enchant_shield(&mut request, 18756, enchant);
    request
}

fn prepare(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err}"))
}

/// Prepared under Forever's rules, through the gate and run: the prepared input and the
/// player's metrics.
fn run(request: &Value, rules: Rules) -> (Value, Value) {
    let value = prepare(request, rules);
    let prepared: PreparedV2 = serde_json::from_value(value.clone()).unwrap();
    check_prepared(&prepared).unwrap_or_else(|err| panic!("{err:?}"));
    let result = simulate_prepared(&prepared).unwrap().result;
    let player = result["raidMetrics"]["parties"][0]["players"][0].clone();
    assert!(player["dps"]["avg"].as_f64().unwrap() > 0.0);
    (value, player)
}

fn damage_proc<'a>(prepared: &'a Value, aura: &str) -> Option<&'a Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "spell_data_damage_proc" && effect["trigger_aura"] == aura)
}

/// The metrics of the player's action with this spell id, summed over every tag and target.
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

/// Each spike deals its row's roll to the attacker on a block, at most once in 5 sec, never
/// missing and never critting.
#[test]
fn each_shield_spike_deals_its_damage_on_a_block() {
    for (enchant, label, spell, average, variance, range) in [
        (
            IRON_SHIELD_SPIKE,
            "Iron Shield Spike",
            IRON_SPIKE_SPELL,
            10.0,
            0.4,
            8.0..=12.0,
        ),
        (
            MITHRIL_SHIELD_SPIKE,
            "Mithril Shield Spike",
            MITHRIL_SPIKE_SPELL,
            18.0,
            0.2222,
            16.0..=20.0,
        ),
        (
            THORIUM_SHIELD_SPIKE,
            "Thorium Shield Spike",
            THORIUM_SPIKE_SPELL,
            25.0,
            0.4,
            20.0..=30.0,
        ),
    ] {
        let request = tank_warrior(enchant);
        let (prepared, player) = run(&request, Rules::Forever);
        let proc = damage_proc(&prepared, label).unwrap_or_else(|| panic!("no proc of {label}"));
        assert_eq!(proc["taken_outcome"], json!(["Block"]), "{label}");
        assert_eq!(proc["outcome"], "always_hit", "{label}");
        assert_eq!(proc["can_crit"], false, "{label}");
        assert_eq!(proc["average"], average, "{label}");
        assert!(
            (proc["variance"].as_f64().unwrap() - variance).abs() < 1e-4,
            "{label}: {proc}"
        );
        let trigger = prepared["player"]["auras"]
            .as_array()
            .unwrap()
            .iter()
            .find(|aura| aura["label"] == label)
            .unwrap();
        assert_eq!(trigger["icd"]["duration_ns"], 5_000_000_000_i64, "{label}");

        let iterations = request["simOptions"]["iterations"].as_f64().unwrap();
        let hits = total(&player, spell, "hits");
        assert!(hits > iterations, "{label}: {hits} hits");
        assert!(hits <= iterations * 24.0, "{label}: {hits} hits");
        assert_eq!(total(&player, spell, "misses"), 0.0, "{label}");
        assert_eq!(total(&player, spell, "crits"), 0.0, "{label}");
        let per_hit = total(&player, spell, "damage") / hits;
        // The roll, less the target's armor and plus the 8 physical damage the fixture's Gift
        // of Arthas adds to every hit the target takes.
        assert!(
            per_hit > *range.start() * 0.8 && per_hit <= *range.end() + 8.0,
            "{label}: {per_hit}"
        );
    }
}

/// The reference's rules give the enchant no effect: no listener, no damage.
#[test]
fn the_reference_rules_ignore_the_enchants() {
    let request = tank_warrior(MITHRIL_SHIELD_SPIKE);
    let (prepared, player) = run(&request, Rules::GoReference);
    assert!(damage_proc(&prepared, "Mithril Shield Spike").is_none());
    assert_eq!(total(&player, MITHRIL_SPIKE_SPELL, "damage"), 0.0);
}
