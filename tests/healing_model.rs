//! Go health.go's healing model on a tank: `applyHealingModel`'s heals and shield, the TMI of
//! its burst window and `GetPresimOptions`' presim.
//!
//! The Go goldens in fixtures/healing-model are the pinned engine's results for accepted
//! fixtures' requests with a model added (tools/healing_model_goldens.py). The exporter marks a
//! healing model as unrepresented, so Rust prepares each request itself, with the Go reference's
//! rules, and the result and first-fight log must match.

use forever_engine::{contracts::prepared_v2::PreparedV2, prepare_json_with, Rules};
use serde::Deserialize;
use serde_json::Value;
use std::{fs, path::Path};

fn family() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/healing-model"
    ))
}

#[derive(Deserialize)]
struct Manifest {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    #[serde(default)]
    go_log: Option<String>,
}

fn request(id: &str) -> Value {
    serde_json::from_slice(&fs::read(family().join(format!("{id}.request.json"))).unwrap()).unwrap()
}

fn prepare(request: &Value) -> Value {
    prepare_json_with(
        &serde_json::to_vec(request).unwrap(),
        "healing-model",
        Rules::GoReference,
    )
    .unwrap_or_else(|err| panic!("{err}"))
}

fn simulate(prepared: Value) -> Value {
    let prepared: PreparedV2 = serde_json::from_value(prepared).unwrap();
    forever_engine::simulate_prepared(&prepared)
        .unwrap_or_else(|err| panic!("{err:?}"))
        .result
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

/// The comparable view of a result, as tools/prepared_v2.py `comparable` writes the goldens:
/// identified lists keyed by ID with an occurrence number, empty containers dropped, no timing
/// or log.
fn comparable(result: &Value) -> Value {
    fn key(id: &Value) -> String {
        let map: std::collections::BTreeMap<String, Value> =
            serde_json::from_value(id.clone()).unwrap();
        serde_json::to_string(&map)
            .unwrap()
            .replace(':', ": ")
            .replace(',', ", ")
    }
    fn is_empty(value: &Value) -> bool {
        matches!(value, Value::Object(map) if map.is_empty())
            || matches!(value, Value::Array(items) if items.is_empty())
    }
    fn view(value: &Value) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), view(v)))
                    .filter(|(_, v)| !is_empty(v))
                    .collect(),
            ),
            Value::Array(items)
                if !items.is_empty() && items.iter().all(|item| item.get("id").is_some()) =>
            {
                let mut keyed = serde_json::Map::new();
                let mut seen = std::collections::BTreeMap::<String, usize>::new();
                for item in items {
                    let base = key(&item["id"]);
                    let count = seen.entry(base.clone()).or_default();
                    *count += 1;
                    let name = if *count == 1 {
                        base
                    } else {
                        format!("{base} #{count}")
                    };
                    let mut rest = item.as_object().unwrap().clone();
                    rest.remove("id");
                    keyed.insert(name, view(&Value::Object(rest)));
                }
                Value::Object(keyed)
            }
            Value::Array(items) => Value::Array(items.iter().map(view).collect()),
            other => other.clone(),
        }
    }
    let mut result = result.as_object().unwrap().clone();
    result.remove("elapsedNs");
    result.remove("logs");
    view(&Value::Object(result))
}

/// Integers exactly, floats within 1e-9 relative, standard deviations as variances: the
/// allowance of tests/classes/mage/prepared_v2.rs.
fn differences(go: &Value, rust: &Value, path: &str, out: &mut Vec<String>) {
    let deviation = |value: Option<&Value>| match value {
        None => Some(0.0),
        Some(Value::String(text)) if text == "NaN" => Some(0.0),
        Some(value) => value.as_f64(),
    };
    match (go, rust) {
        (Value::Object(a), Value::Object(b)) => {
            let keys: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
            for key in keys {
                let (x, y) = (a.get(key), b.get(key));
                let mean_key = match key.strip_suffix("Stdev") {
                    Some(stem) => Some(format!("{stem}Avg")),
                    None if key.as_str() == "stdev" => Some("avg".to_string()),
                    None => None,
                };
                if let Some(mean_key) = mean_key {
                    let mean = a.get(&mean_key).and_then(Value::as_f64).unwrap_or(0.0);
                    let (x, y) = (deviation(x).unwrap(), deviation(y).unwrap());
                    if (x * x - y * y).abs() > 1e-9 * (mean * mean).max(1.0) {
                        out.push(format!("{path}/{key}: Go {x}, Rust {y}"));
                    }
                    continue;
                }
                match (x, y) {
                    (Some(x), Some(y)) => differences(x, y, &format!("{path}/{key}"), out),
                    _ => out.push(format!("{path}/{key}: present on one side only")),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (index, (x, y)) in a.iter().zip(b).enumerate() {
                differences(x, y, &format!("{path}/{index}"), out);
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            let (x, y) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            let integral = (a.is_i64() || a.is_u64()) && (b.is_i64() || b.is_u64());
            let tolerance = if integral {
                0.0
            } else {
                1e-9 * x.abs().max(y.abs()).max(1.0)
            };
            if (x - y).abs() > tolerance {
                out.push(format!("{path}: Go {x}, Rust {y}"));
            }
        }
        (a, b) if a == b => {}
        (a, b) => out.push(format!("{path}: Go {a}, Rust {b}")),
    }
}

/// Every case matches the pinned Go engine: chance of death, the model's health metrics and
/// shield uptime, damage taken, TMI, and for the first case the first fight's log line by line.
#[test]
fn healing_models_match_pinned_go_goldens() {
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(family().join("manifest.json")).unwrap()).unwrap();
    assert!(!manifest.cases.is_empty());
    let mut failures = Vec::new();
    for case in &manifest.cases {
        let result = simulate(prepare(&request(&case.id)));
        let golden: Value = serde_json::from_slice(
            &fs::read(family().join(format!("{}.go-result.json", case.id))).unwrap(),
        )
        .unwrap();
        let mut found = Vec::new();
        differences(&golden, &comparable(&result), "", &mut found);
        if !found.is_empty() {
            failures.push(format!(
                "{}:\n  {}",
                case.id,
                found[..found.len().min(20)].join("\n  ")
            ));
            continue;
        }
        if let Some(log) = &case.go_log {
            let expected = fs::read_to_string(family().join(log)).unwrap();
            let actual = result["logs"].as_str().unwrap_or_default();
            let mismatch = expected
                .lines()
                .zip(actual.lines())
                .enumerate()
                .find(|(_, (go, rust))| go != rust);
            if let Some((line, (go, rust))) = mismatch {
                failures.push(format!(
                    "{} log line {}:\n  Go:   {go}\n  Rust: {rust}",
                    case.id,
                    line + 1
                ));
            } else if expected.lines().count() != actual.lines().count() {
                failures.push(format!("{} log length differs", case.id));
            }
        }
    }
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
