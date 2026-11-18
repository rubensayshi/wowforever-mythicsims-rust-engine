//! Blizzard's WoW Forever hotfixes of 17 November 2026, the line that changes a fight, under
//! Forever's rules only:
//!
//! - Shaman: "Fixed an issue where Flurry incorrectly lost stacks when Windfury totem attacks
//!   landed." The Windfury Totem strike, the party's or the shaman's own, spends no charge of
//!   the Shaman's Flurry and leaves the charge cooldown alone. Its crit still starts Flurry, as
//!   any melee crit does: the post speaks of lost stacks only.
//!
//! Until the hotfix the strike, a copy of the main hand auto, spent a charge as a white swing
//! does whenever the 500 ms charge cooldown was ready: right after a white crit that had just
//! refreshed Flurry, or after a Stormstrike that fired the totem. The Go reference keeps its
//! own rules.

use forever_engine::{prepare_json_with, simulate_prepared, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

/// The tags of the party totem's strike and of the shaman's own totem's.
const PARTY_STRIKE: i64 = 25584;
const OWN_STRIKE: i64 = 10610;
/// Flurry's aura and Windfury Totem's highest rank.
const FLURRY: i64 = 16257;
const WINDFURY_TOTEM: i64 = 10614;
/// The live Enhancement reference's talents on the launch tree, which only Forever's rules take.
const ENHANCEMENT: &str = "053303102-053030310051002251-03";

/// The production Enhancement request: a two-hand axe with Rockbiter Weapon, Stormstrike in the
/// rotation and the party's Windfury Totem.
fn request(rules: Rules, iterations: u32, log: bool) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/mage/prepared-v2/production-enhancement-shaman.request.json");
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(iterations);
    request["simOptions"]["debugFirstIteration"] = json!(log);
    request["raid"]["parties"][0]["buffs"]["windfuryTotem"] = json!(true);
    if rules == Rules::Forever {
        request["raid"]["parties"][0]["players"][0]["talentsString"] = json!(ENHANCEMENT);
    }
    request
}

/// The same request without the party's totem, the shaman keeping its own Windfury Totem up.
fn own_totem_request(iterations: u32, log: bool) -> Value {
    let mut request = request(Rules::Forever, iterations, log);
    request["raid"]["parties"][0]["buffs"]["windfuryTotem"] = json!(false);
    let list = request["raid"]["parties"][0]["players"][0]["rotation"]["priorityList"]
        .as_array_mut()
        .unwrap();
    list.insert(
        0,
        json!({"action": {"castSpell": {"spellId": {"spellId": WINDFURY_TOTEM}},
            "condition": {"cmp": {"lhs": {"totemRemainingTime": {"totemType": "Air"}},
                "op": "OpLe", "rhs": {"const": {"val": "0s"}}}}}}),
    );
    request
}

fn prepare(request: &Value, rules: Rules) -> Value {
    prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err:?}"))
}

/// The run's result: the player's metrics are under `raidMetrics`, the first iteration's log
/// under `logs`.
fn run(prepared: &Value) -> Value {
    let prepared = serde_json::from_value(prepared.clone()).unwrap();
    simulate_prepared(&prepared).unwrap().result
}

fn flurry(prepared: &Value) -> &Value {
    prepared["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "flurry")
        .expect("the flurry effect")
}

fn flurry_mut(prepared: &mut Value) -> &mut Value {
    prepared["effects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|effect| effect["kind"] == "flurry")
        .expect("the flurry effect")
}

/// Flurry's uptime in seconds an iteration.
fn flurry_uptime(result: &Value) -> f64 {
    result["raidMetrics"]["parties"][0]["players"][0]["auras"]
        .as_array()
        .unwrap()
        .iter()
        .find(|aura| aura["id"]["spellId"].as_i64() == Some(FLURRY))
        .and_then(|aura| aura["uptimeSecondsAvg"].as_f64())
        .expect("Flurry")
}

/// A logged attack: when it was rolled and whether it crit.
#[derive(Clone, Copy, Debug)]
struct Attack {
    time: f64,
    crit: bool,
}

/// What the first iteration's log shows of Flurry: the white swings, the totem strikes of this
/// tag and the moments a charge was spent.
struct Logged {
    swings: Vec<Attack>,
    strikes: Vec<Attack>,
    spent: Vec<f64>,
}

fn logged(result: &Value, strike_tag: i64) -> Logged {
    let log = result["logs"].as_str().expect("the first iteration's log");
    let mut logged = Logged {
        swings: Vec::new(),
        strikes: Vec::new(),
        spent: Vec::new(),
    };
    let strike = format!("{{OtherID: 3, Tag: {strike_tag}}}");
    let charges = format!("{{SpellID: {FLURRY}}} stacks: ");
    let gained = format!("Aura gained: {{SpellID: {FLURRY}}}");
    let faded = format!("Aura faded: {{SpellID: {FLURRY}}}");
    // Whether Flurry has faded: its charges are then cleared, which is no charge spent. That
    // is how its 15 seconds running out and the fight's end read in the log.
    let mut down = true;
    for line in log.lines() {
        let Some(time) = time_of(line) else {
            continue;
        };
        if line.contains(&gained) {
            down = false;
            continue;
        }
        if line.contains(&faded) {
            down = true;
            continue;
        }
        if let Some((_, change)) = line.split_once(&charges) {
            let (from, to) = change.split_once(" --> ").expect("a stack change");
            if !down && to.trim().parse::<i32>().unwrap() < from.trim().parse::<i32>().unwrap() {
                logged.spent.push(time);
            }
            continue;
        }
        // An attack's outcome line, not its cast or its damage breakdown.
        if !line.contains("(Threat: ") || line.contains("[DEBUG]") {
            continue;
        }
        let attack = Attack {
            time,
            crit: line.contains("} Crit for "),
        };
        if line.contains("{OtherID: 3, Tag: 1}") || line.contains("{OtherID: 3, Tag: 2}") {
            logged.swings.push(attack);
        } else if line.contains(&strike) {
            logged.strikes.push(attack);
        }
    }
    logged
}

/// Flurry's handler runs one 10 ms batch window after the hit, and the log keeps two decimals.
fn same_batch(hit: f64, handled: f64) -> bool {
    (-0.001..=0.021).contains(&(handled - hit))
}

/// Two attacks of one moment, as a strike and the swing that fired it.
fn same_moment(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.011
}

/// When a log line happened.
fn time_of(line: &str) -> Option<f64> {
    line.strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
        .and_then(|(time, _)| time.parse::<f64>().ok())
}

/// Every charge the log shows spent follows a white swing that did not crit, and at least one
/// strike that did not crit landed beside a white crit, where it spent a charge before the
/// hotfix. The request wields a two-hand weapon, so no off hand swing shares a moment with the
/// strike.
fn assert_only_white_swings_spend(result: &Value, strike_tag: i64) {
    let logged = logged(result, strike_tag);
    assert!(!logged.spent.is_empty(), "no Flurry charge was spent");
    for &spent in &logged.spent {
        assert!(
            logged
                .swings
                .iter()
                .any(|swing| !swing.crit && same_batch(swing.time, spent)),
            "the charge spent at {spent} follows no white swing"
        );
    }
    let exposed = logged
        .strikes
        .iter()
        .filter(|strike| {
            !strike.crit
                && logged
                    .swings
                    .iter()
                    .any(|swing| swing.crit && same_moment(swing.time, strike.time))
        })
        .count();
    assert!(
        exposed > 0,
        "no strike beside a white crit in {} strikes",
        logged.strikes.len()
    );
}

/// The prepared `flurry` effect carries the rule under Forever's rules only.
#[test]
fn flurry_carries_the_rule_under_forevers_rules_only() {
    let forever = prepare(&request(Rules::Forever, 10, false), Rules::Forever);
    assert_eq!(flurry(&forever)["totem_strike_spends_none"], json!(true));
    let reference = prepare(&request(Rules::GoReference, 10, false), Rules::GoReference);
    assert!(flurry(&reference).get("totem_strike_spends_none").is_none());
}

/// The party totem's strike spends no charge: in the first iteration's log every charge spent
/// follows a white swing that did not crit.
#[test]
fn the_partys_totem_strike_spends_no_flurry_charge() {
    let prepared = prepare(&request(Rules::Forever, 1, true), Rules::Forever);
    assert_only_white_swings_spend(&run(&prepared), PARTY_STRIKE);
}

/// The shaman's own totem's strike follows the same rule.
#[test]
fn the_own_totems_strike_spends_no_flurry_charge() {
    let prepared = prepare(&own_totem_request(1, true), Rules::Forever);
    assert_eq!(flurry(&prepared)["totem_strike_spends_none"], json!(true));
    assert_only_white_swings_spend(&run(&prepared), OWN_STRIKE);
}

/// Without the rule the strike spends charges, so Flurry is up less.
#[test]
fn flurry_stays_up_longer_under_the_rule() {
    let spared = prepare(&request(Rules::Forever, 300, false), Rules::Forever);
    let mut spending = spared.clone();
    flurry_mut(&mut spending)["totem_strike_spends_none"] = json!(false);
    let (spared, spending) = (flurry_uptime(&run(&spared)), flurry_uptime(&run(&spending)));
    assert!(spared > spending, "{spared} vs {spending}");
}

/// The strike's crit still starts Flurry: a strike crit beside no white crit is followed by
/// Flurry's gain or its refresh.
#[test]
fn the_totem_strikes_crit_still_starts_flurry() {
    let prepared = prepare(&request(Rules::Forever, 1, true), Rules::Forever);
    let result = run(&prepared);
    let log = result["logs"].as_str().unwrap();
    let logged = logged(&result, PARTY_STRIKE);
    let gained = format!("Aura gained: {{SpellID: {FLURRY}}}");
    let refreshed = format!("Aura refreshed: {{SpellID: {FLURRY}}}");
    let starts: Vec<f64> = log
        .lines()
        .filter(|line| line.contains(&gained) || line.contains(&refreshed))
        .filter_map(time_of)
        .collect();
    let alone: Vec<&Attack> = logged
        .strikes
        .iter()
        .filter(|strike| {
            strike.crit
                && !logged
                    .swings
                    .iter()
                    .any(|swing| swing.crit && same_moment(swing.time, strike.time))
        })
        .collect();
    assert!(!alone.is_empty(), "no strike crit beside no white crit");
    for strike in alone {
        assert!(
            starts.iter().any(|&start| same_batch(strike.time, start)),
            "the strike crit at {} started no Flurry",
            strike.time
        );
    }
}
