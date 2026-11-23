//! A sequence's wait step (Go `APLActionWait` inside `APLActionSequence`).
//!
//! Go builds each action in a sequence's `actions` with `newAPLAction`, so a wait is a step like
//! a cast: when the sequence reaches it, the wait takes control of the rotation for its duration
//! and the sequence moves on to its next step. Rust refused the whole rotation ("sequence action
//! wait is unsupported"), which failed a player's custom Enhancement rotation on 23 Nov 2026.

use forever_engine::{
    contracts::prepared_v2::PreparedV2, prepare_json_with, simulate_prepared, Rules,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

const AFFLICTION: &str =
    "fixtures/mage/prepared-v2/production-affliction-warlock-wrack.request.json";
const SHADOW_BOLT: i64 = 25307;

fn request() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(AFFLICTION);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn bolt() -> Value {
    json!({"castSpell": {"spellId": {"spellId": SHADOW_BOLT}}})
}

/// The first iteration's log of a 30 s fight whose whole rotation is this one sequence.
fn log(actions: Value, rules: Rules) -> String {
    let mut request = request();
    request["raid"]["parties"][0]["players"][0]["rotation"] = json!({
        "type": "TypeAPL",
        "priorityList": [{"action": {"sequence": {"name": "s", "actions": actions}}}],
    });
    request["encounter"]["duration"] = json!(30);
    request["encounter"]["durationVariation"] = json!(0);
    request["simOptions"]["iterations"] = json!(1);
    request["simOptions"]["debugFirstIteration"] = json!(true);
    request["simOptions"]["randomSeed"] = json!("7");
    let prepared = prepare_json_with(request.to_string().as_bytes(), "rust", rules)
        .unwrap_or_else(|err| panic!("{err:?}"));
    let prepared: PreparedV2 = serde_json::from_value(prepared).unwrap();
    let report = simulate_prepared(&prepared).unwrap();
    report.result["logs"].as_str().unwrap().to_string()
}

fn time(line: &str) -> f64 {
    line[1..line.find(']').unwrap()].parse().unwrap()
}

/// The times the player starts casting Shadow Bolt, which the log names by the id of the rank
/// it casts (11661 under Forever's rules, 25307 under the Go reference's).
fn bolt_starts(log: &str) -> Vec<f64> {
    log.lines()
        .filter(|line| {
            line.contains("] Casting {SpellID: 11661}")
                || line.contains("] Casting {SpellID: 25307}")
        })
        .map(time)
        .collect()
}

#[test]
fn a_wait_step_holds_the_sequence_for_its_duration() {
    for rules in [Rules::Forever, Rules::GoReference] {
        let plain = bolt_starts(&log(json!([bolt(), bolt()]), rules));
        let waited = bolt_starts(&log(
            json!([bolt(), {"wait": {"duration": {"const": {"val": "5s"}}}}, bolt()]),
            rules,
        ));
        assert_eq!(plain.len(), 2, "{plain:?}");
        assert_eq!(waited.len(), 2, "{waited:?}");
        assert_eq!(plain[0], waited[0], "the first bolt is the same");
        let cast = plain[1] - plain[0];
        let gap = waited[1] - waited[0];
        // As in Go's DoNextAction, the wait is ready as soon as the first bolt starts (a wait
        // does not wait for the cast), so it runs from then: the second bolt starts 5 s after
        // the first one started, where without the wait it follows the first cast at once.
        assert!(
            cast < 4.0,
            "without the wait the bolts follow each other: {cast}"
        );
        assert!(
            (gap - 5.0).abs() < 0.01,
            "the wait holds the sequence 5 s: gap {gap}"
        );
    }
}

#[test]
fn a_wait_step_without_a_duration_is_dropped() {
    // Go's newActionWait builds no action without a value, and the sequence drops the step.
    let plain = bolt_starts(&log(json!([bolt(), bolt()]), Rules::Forever));
    let dropped = bolt_starts(&log(json!([bolt(), {"wait": {}}, bolt()]), Rules::Forever));
    assert_eq!(plain, dropped);
}

#[test]
fn a_wait_step_keeps_its_own_condition() {
    // A step's condition is its own action's: one that never holds leaves the sequence on that
    // step. (A constant false prunes the step instead, as it does a cast.)
    let never = json!({"cmp": {"op": "OpGt", "lhs": {"currentTime": {}}, "rhs": {"const": {"val": "100s"}}}});
    let held = bolt_starts(&log(
        json!([bolt(), {"wait": {"duration": {"const": {"val": "1s"}}}, "condition": never}, bolt()]),
        Rules::Forever,
    ));
    assert_eq!(held.len(), 1, "{held:?}");
}
