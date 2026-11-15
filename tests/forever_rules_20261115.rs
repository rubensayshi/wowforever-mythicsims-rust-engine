//! Forever's rules of 15 November 2026, each from live client rows or counted combat logs:
//!
//! - Windfury Totem's proc is a weapon strike since the hotfixes of 13 November 2026. The live
//!   rows (Wowhead Forever, 15 November 2026): 10610 deals weapon damage with a 100 ms cooldown
//!   and applies no attack power aura, and the party aura 10612 carries the 246 attack power as
//!   a dummy. The strike rolls the special attack table, grants no charges, gives Maelstrom
//!   Weapon no stack, and an Execute rolls the totem from its cast attack as well as its damage.
//! - A damage shield answers no hit blocked in full.
//! - A direct hit taken of a school other than Physical gives twice the rage.
//! - Live item rows from the Forever tooltips of 15 November 2026.
//!
//! The counts are in the comments at each rule. The Go reference keeps its own rules.

use forever_engine::{prepare_json_with, simulate_prepared, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// The tag of the party totem's strike, the main hand auto's copy.
const TOTEM_STRIKE: i64 = 25584;
/// The live Fury and Protection references' talents on the launch tree.
const FURY: &str = "30303213002-050530045120100231-2";
const PROTECTION: &str = "-05250035-250033120310021251";
const ENHANCEMENT: &str = "053303102-053030310051002251-03";

fn request(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("fixtures/mage/prepared-v2/{name}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(100);
    request["simOptions"]["debugFirstIteration"] = json!(false);
    request
}

/// A production request with the live reference's talents, which only Forever's rules take,
/// and the party's Windfury Totem up.
fn production(name: &str, talents: &str, rules: Rules) -> Value {
    let mut request = request(name);
    request["raid"]["parties"][0]["buffs"]["windfuryTotem"] = json!(true);
    if rules == Rules::Forever {
        request["raid"]["parties"][0]["players"][0]["talentsString"] = json!(talents);
    }
    prepared(&request, rules)
}

fn prepared(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err:?}"))
}

fn run(prepared: &Value) -> Value {
    let prepared = serde_json::from_value(prepared.clone()).unwrap();
    let report = simulate_prepared(&prepared).unwrap();
    report.result["raidMetrics"]["parties"][0]["players"][0].clone()
}

fn effects<'a>(prepared: &'a Value, kind: &str) -> Vec<&'a Value> {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|effect| effect["kind"] == kind)
        .collect()
}

fn effect<'a>(prepared: &'a Value, kind: &str) -> &'a Value {
    effects(prepared, kind)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no {kind} effect"))
}

/// The first target's metrics of the action with this id.
fn action(player: &Value, matches: impl Fn(&Value) -> bool) -> Value {
    player["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| matches(&action["id"]))
        .map(|action| action["targets"][0].clone())
        .unwrap_or_else(|| panic!("no such action"))
}

fn totem_strike(id: &Value) -> bool {
    id["otherId"] == "OtherActionAttack" && id["tag"].as_i64() == Some(TOTEM_STRIKE)
}

fn count(metrics: &Value, field: &str) -> u64 {
    metrics[field].as_u64().unwrap_or(0)
}

/// The proc casts one main hand strike with rank 3's 246 attack power added, on the special
/// table: it never glances and grants no charges. The reference keeps the extra white swing and
/// the two charges.
#[test]
fn windfury_totem_proc_is_a_weapon_strike() {
    let forever = production("production-warrior", FURY, Rules::Forever);
    assert_eq!(
        effect(&forever, "windfury_totem")["strike_attack_power"],
        json!(246.0)
    );
    let strikes = action(&run(&forever), totem_strike);
    assert!(count(&strikes, "casts") > 1000, "{strikes}");
    assert!(count(&strikes, "crits") > 0, "{strikes}");
    assert_eq!(count(&strikes, "glances"), 0, "{strikes}");

    let reference = production("production-warrior", FURY, Rules::GoReference);
    assert!(effect(&reference, "windfury_totem")
        .get("strike_attack_power")
        .is_none());
    let swings = action(&run(&reference), totem_strike);
    assert!(count(&swings, "glances") > 0, "{swings}");
}

/// Combat logs taken after the hotfix of 14 November 2026: the totem's strike followed 267 of
/// 846 landed Executes (31.6%, where one roll gives 20%) and 35 of 185 dodged ones (18.9%), so
/// the cast's own attack rolls it as well as the damage.
#[test]
fn execute_rolls_the_totem_from_its_cast_attack() {
    let forever = production("production-warrior", FURY, Rules::Forever);
    assert_eq!(
        effect(&forever, "execute")["cast_attack_rolls_windfury_totem"],
        json!(true)
    );
    let reference = production("production-warrior", FURY, Rules::GoReference);
    assert!(effect(&reference, "execute")
        .get("cast_attack_rolls_windfury_totem")
        .is_none());
}

/// Combat logs taken after the hotfixes of 13 November 2026: the totem's strike gave Maelstrom
/// Weapon no stack, 0 of 140 landed strikes of ranks 1 and 2 and 0 of 121 of rank 3. White
/// swings keep their chance.
#[test]
fn the_totem_strike_gives_maelstrom_weapon_no_stack() {
    let chances = |rules: Rules| {
        let prepared = production("production-enhancement-shaman", ENHANCEMENT, rules);
        let spells = prepared["player"]["spells"].as_array().unwrap().clone();
        let heard: Vec<i64> = effect(&prepared, "maelstrom_weapon")["chances"]
            .as_array()
            .unwrap()
            .iter()
            .map(|chance| spells[chance["spell"].as_u64().unwrap() as usize]["action_id"].clone())
            .filter(|id| id["other_id"] == "OtherActionAttack")
            .map(|id| id["tag"].as_i64().unwrap_or(0))
            .collect();
        heard
    };
    let forever = chances(Rules::Forever);
    assert!(forever.contains(&1), "{forever:?}");
    assert!(!forever.contains(&TOTEM_STRIKE), "{forever:?}");
    assert!(chances(Rules::GoReference).contains(&TOTEM_STRIKE));
}

/// A protection warrior's combat logs of levels 45 to 59 (clients 1.60.1.70862 to
/// 1.60.1.70938): Thorns answered 1 of 8,334 hits blocked in full, and Razorsteel Shoulders'
/// shield 34 of 5,281, against 3,835 of 3,840 partial blocks and 15,042 of 15,069 hits.
#[test]
fn a_damage_shield_answers_no_full_block() {
    let thorns = |rules: Rules| {
        let mut request = request("production-protection-warrior");
        request["raid"]["buffs"]["thorns"] = json!(true);
        // A weak attacker, whose swings a shield blocks in full now and then. A level 63 boss's
        // swings are never blocked in full, so the rule moves no board.
        request["encounter"]["targets"][0]["minBaseDamage"] = json!(20);
        if rules == Rules::Forever {
            request["raid"]["parties"][0]["players"][0]["talentsString"] = json!(PROTECTION);
        }
        prepared(&request, rules)
    };
    let forever = thorns(Rules::Forever);
    let shields: Vec<&Value> = effects(&forever, "spell_data_damage_proc")
        .into_iter()
        .filter(|effect| effect["struck"] == json!(true))
        .collect();
    assert!(!shields.is_empty(), "no damage shield");
    assert!(shields
        .iter()
        .all(|shield| shield["skips_full_blocks"] == json!(true)));
    let reference = thorns(Rules::GoReference);
    assert!(effects(&reference, "spell_data_damage_proc")
        .iter()
        .all(|shield| shield.get("skips_full_blocks").is_none()));

    // The same fight answers more hits without the rule.
    let id = forever["player"]["spells"][shields[0]["spell"].as_u64().unwrap() as usize]
        ["action_id"]
        .clone();
    let answers = |prepared: &Value| -> u64 {
        let answered = action(&run(prepared), |action| {
            action["spellId"] == id["spell_id"]
                && action["tag"].as_i64().unwrap_or(0) == id["tag"].as_i64().unwrap_or(0)
        });
        count(&answered, "hits") + count(&answered, "crits")
    };
    let mut every_hit = forever.clone();
    for effect in every_hit["effects"].as_array_mut().unwrap() {
        if let Some(effect) = effect.as_object_mut() {
            effect.remove("skips_full_blocks");
        }
    }
    let skipping = answers(&forever);
    assert!(skipping > 0);
    assert!(answers(&every_hit) > skipping);
}

/// A warrior's combat logs of levels 27 to 38 (clients 1.60.1.70862 to 1.60.1.70938), hits from
/// rage-using mobs with Defensive Stance divided out: 3.85 rage per point of `damage / level
/// factor` on 26 arcane swings and 3.83 on 9 fire swings, against 1.96 on 3,702 physical ones.
#[test]
fn a_non_physical_hit_taken_gives_twice_the_rage() {
    let forever = production("production-protection-warrior", PROTECTION, Rules::Forever);
    assert_eq!(
        effect(&forever, "rage_bar")["spell_damage_taken_rage_multiplier"],
        json!(2.0)
    );
    let reference = production(
        "production-protection-warrior",
        PROTECTION,
        Rules::GoReference,
    );
    assert!(effect(&reference, "rage_bar")
        .get("spell_damage_taken_rage_multiplier")
        .is_none());
}

/// The live rows read from Wowhead's Forever tooltips on 15 November 2026.
#[test]
fn live_rows_of_15_november() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/forever-live-items.jsonl");
    let text = fs::read_to_string(path).unwrap();
    let stats = |id: i64| -> Value {
        text.lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .find(|row| row["id"] == id)
            .unwrap_or_else(|| panic!("no live row {id}"))["scaling_options"]["0"]["stats"]
            .clone()
    };
    // Diabolic Mantle: +16 Stamina, 5 mana per 5 sec and up to 17 Fire damage.
    assert_eq!(stats(18757), json!({"2": 16, "7": 17, "30": 66, "34": 5}));
    // Bad Hat: 37 healing and 12 damage, no Shadow damage.
    assert_eq!(
        stats(260212),
        json!({"3": 17, "4": 37, "5": 12, "16": 17, "30": 65, "31": 0})
    );
    // Souvenir Sea Shell: +10 Stamina.
    assert_eq!(stats(274749), json!({"2": 10}));
}
