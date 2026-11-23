//! Go health.go's healing model on a tank: `applyHealingModel`'s heals and shield, the TMI of
//! its burst window and `GetPresimOptions`' presim.
//!
//! The Go goldens in fixtures/healing-model are the pinned engine's results for accepted
//! fixtures' requests with a model added (tools/healing_model_goldens.py). The exporter marks a
//! healing model as unrepresented, so Rust prepares each request itself, with the Go reference's
//! rules, and the result and first-fight log must match.

#[path = "support/go_golden.rs"]
mod go_golden;

use forever_engine::{prepare_json_with, Rules};
use go_golden::{prepare, simulate};
use serde_json::Value;
use std::path::Path;

fn family() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/healing-model"
    ))
}

fn request(id: &str) -> Value {
    go_golden::request(family(), id)
}

fn player(result: &Value) -> &Value {
    &result["raidMetrics"]["parties"][0]["players"][0]
}

fn model_effect(prepared: &mut Value) -> &mut Value {
    prepared["effects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|effect| effect["kind"] == "healing_model")
        .expect("a healing model effect")
}

/// Every case matches the pinned Go engine: chance of death, the model's health metrics and
/// shield uptime, damage taken, TMI, and for the first case the first fight's log line by line.
#[test]
fn healing_models_match_pinned_go_goldens() {
    let failures = go_golden::check_family(family());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The model's heals in the first fight's log: each heal's time and amount.
fn heals(result: &Value) -> Vec<(f64, f64)> {
    result["logs"]
        .as_str()
        .unwrap()
        .lines()
        .filter(|line| line.contains(" health from {OtherID: 12} "))
        .map(|line| {
            let time: f64 = line[1..line.find(']').unwrap()].parse().unwrap();
            let amount = line.split("Gained ").nth(1).unwrap();
            let amount: f64 = amount[..amount.find(' ').unwrap()].parse().unwrap();
            (time, amount)
        })
        .collect()
}

/// A median of 2 s with 1 s of variation heals every 1 to 3 s, on both sides of the median, each
/// heal `hps` times the time since the last. The heals' times add up to the last heal's time
/// from the 0:00 run, which only rolls.
#[test]
fn heals_land_within_the_cadence_bounds() {
    let mut prepared = prepare(&request("tank-heals"));
    let effect = model_effect(&mut prepared);
    effect["absorb_frac"] = 0.0.into();
    effect.as_object_mut().unwrap().remove("absorb_aura");
    let hps = effect["hps"].as_f64().unwrap();
    let heals = heals(&simulate(prepared));
    assert!(heals.len() > 30, "{} heals", heals.len());
    let gaps: Vec<f64> = heals.iter().map(|(_, amount)| amount / hps).collect();
    for gap in &gaps {
        assert!((1.0..=3.0).contains(gap), "a heal {gap} s after the last");
    }
    assert!(gaps.iter().any(|gap| *gap < 2.0) && gaps.iter().any(|gap| *gap > 2.0));
    let last = heals.last().unwrap().0;
    let total: f64 = gaps.iter().sum();
    assert!((total - last).abs() < 0.006, "{total} s of heals by {last}");
}

/// The same seed with half the healing taken and 0.8 of the external healing taken: the first
/// heal lands at the same moment for 0.4 of the amount.
#[test]
fn healing_taken_multipliers_scale_each_heal() {
    let mut prepared = prepare(&request("tank-heals"));
    let base = heals(&simulate(prepared.clone()))[0];
    let effect = model_effect(&mut prepared);
    effect["healing_taken_multiplier"] = 0.5.into();
    effect["external_healing_taken_multiplier"] = 0.8.into();
    let scaled = heals(&simulate(prepared))[0];
    assert_eq!(scaled.0, base.0);
    assert!(
        (scaled.1 - 0.4 * base.1).abs() < 0.002,
        "{scaled:?} {base:?}"
    );
}

/// Without a healer the tank dies in every fight; a healer of 1500 healing a second keeps it
/// alive in most.
#[test]
fn a_healer_lowers_the_chance_of_death() {
    let mut without = request("tank-heals");
    without["raid"]["parties"][0]["players"][0]
        .as_object_mut()
        .unwrap()
        .remove("healingModel");
    let alone = simulate(prepare(&without));
    assert_eq!(player(&alone)["chanceOfDeath"], 1.0);
    assert!(
        player(&alone)["tmi"].get("avg").is_none(),
        "no TMI without a model"
    );
    let mut healed = request("tank-heals");
    healed["raid"]["parties"][0]["players"][0]["healingModel"]["hps"] = 1500.into();
    let result = simulate(prepare(&healed));
    let chance = player(&result)["chanceOfDeath"].as_f64().unwrap_or(0.0);
    assert!(chance < 0.5, "chance of death {chance}");
    assert!(player(&result)["tmi"]["avg"].as_f64().unwrap() > 0.0);
}

/// Go reads a healing model only for a player a target swings at: a DPS warrior with one
/// prepares exactly as without it.
#[test]
fn a_player_nobody_tanks_ignores_the_model() {
    let with = request("not-tanking");
    let mut without = with.clone();
    without["raid"]["parties"][0]["players"][0]
        .as_object_mut()
        .unwrap()
        .remove("healingModel");
    let mut with = prepare(&with);
    let without = prepare(&without);
    with["request_sha256"] = without["request_sha256"].clone();
    assert_eq!(with, without);
}

/// The Protection Warrior applies Inspiration at the model's uptime in Go, which Rust does not:
/// the request is refused with the healing model's code.
#[test]
fn a_protection_warriors_inspiration_uptime_is_refused() {
    let mut request = request("tank-heals");
    request["raid"]["parties"][0]["players"][0]["healingModel"]["inspirationUptime"] = 0.5.into();
    let refused = prepare_json_with(
        &serde_json::to_vec(&request).unwrap(),
        "healing-model",
        Rules::GoReference,
    )
    .expect_err("refused");
    assert!(
        refused.to_string().contains("Inspiration uptime"),
        "{refused}"
    );
}
