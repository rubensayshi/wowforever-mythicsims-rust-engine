//! Go sim.go and target.go's fights that end on health (`encounter.useHealth`): the fight ends
//! once the targets have taken the sum of their health, with no time limit, the execute phases
//! follow the share of that health dealt, and a presim measures the fight's length.
//!
//! The Go goldens in fixtures/health-fight are the pinned engine's results for accepted fixtures'
//! requests turned into health fights (tools/health_fight_goldens.py). The exporter marks a health
//! fight as unrepresented, so Rust prepares each request itself, with the Go reference's rules,
//! and the result and first-fight logs must match.

#[path = "support/go_golden.rs"]
mod go_golden;

use forever_engine::{simulate_raid, RaidError, Rules};
use go_golden::{prepare, simulate};
use serde_json::{json, Value};
use std::path::Path;

fn family() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/health-fight"
    ))
}

fn request(id: &str) -> Value {
    go_golden::request(family(), id)
}

fn player(result: &Value) -> &Value {
    &result["raidMetrics"]["parties"][0]["players"][0]
}

/// The casts of a spell over every iteration.
fn casts(result: &Value, spell_ids: &[i64]) -> i64 {
    player(result)["actions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| {
            action["id"]["spellId"]
                .as_i64()
                .is_some_and(|id| spell_ids.contains(&id))
        })
        .flat_map(|action| action["targets"].as_array().unwrap().iter())
        .map(|target| target["casts"].as_i64().unwrap_or(0))
        .sum()
}

#[test]
fn health_fights_match_pinned_go_goldens() {
    let failures = go_golden::check_family(family());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A health fight runs on a guessed ten minutes until the presim measures it, and ends at the sum
/// of the request's target health: three copies of 30000.
#[test]
fn the_fight_ends_at_the_targets_health() {
    let prepared = prepare(&request("protection-warrior-3-targets"));
    let encounter = &prepared["encounter"];
    assert_eq!(encounter["end_fight_at_health"], 90000.0);
    assert_eq!(encounter["duration_ns"], 600_000_000_000i64);
    assert_eq!(encounter["duration_is_estimate"], true);
}

/// The request's 60 seconds are no limit: a mob with fifty times the health takes the warrior
/// past them, and the reported length is the time the fight took.
#[test]
fn the_duration_is_no_limit() {
    let mut request = request("protection-warrior-level-60-mob");
    request["encounter"]["targets"][0]["stats"][32] = json!(206000);
    request["simOptions"]["iterations"] = json!(20);
    let result = simulate(prepare(&request));
    let length = result["avgIterationDuration"].as_f64().unwrap();
    assert!(length > 120.0, "{length} s a fight");
}

/// The execute phases follow the health dealt, not the request's proportions of the time: with
/// every proportion at zero, a timed fight never reaches the execute phase, while a health fight
/// still casts Execute once 80% of the health is gone.
#[test]
fn execute_follows_the_health_left() {
    const EXECUTE: [i64; 5] = [5308, 20658, 20660, 20661, 20662];
    let mut request = request("fury-warrior-execute");
    for field in [
        "executeProportion20",
        "executeProportion25",
        "executeProportion35",
        "executeProportion45",
        "executeProportion90",
    ] {
        request["encounter"][field] = json!(0);
    }
    request["simOptions"]["iterations"] = json!(20);
    request["simOptions"]["debugFirstIteration"] = json!(false);
    let health = simulate(prepare(&request));
    assert!(casts(&health, &EXECUTE) > 0);
    request["encounter"]["useHealth"] = json!(false);
    let timed = simulate(prepare(&request));
    assert_eq!(casts(&timed, &EXECUTE), 0);
}

/// A raid runs every member to the first member's length, which a health fight does not have
/// yet: the raid refuses it.
#[test]
fn a_raid_refuses_a_health_fight() {
    let mut request = request("protection-warrior-level-60-mob");
    // The fixture's talents in the client's order, which Forever's rules read.
    request["raid"]["parties"][0]["players"][0]["talentsString"] =
        json!("-05052035-250033120310021251");
    let input = json!({ "schema": 1, "request": request });
    match simulate_raid(&serde_json::to_vec(&input).unwrap(), Rules::Forever, false) {
        Err(RaidError::Refused(refusals)) => {
            assert!(refusals
                .iter()
                .any(|refusal| refusal.reason.contains("health-based fight")));
        }
        other => panic!("expected a refusal, got {:?}", other.map(|_| ())),
    }
}
