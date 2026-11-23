//! Go goldens of requests Rust prepares itself (the exporter marks them unrepresented): a family
//! directory of `<case>.request.json` and `<case>.go-result.json`, with an optional first-fight
//! log, that a tools/*_goldens.py script writes from the pinned Go engine.

#![allow(dead_code)]

use forever_engine::{contracts::prepared_v2::PreparedV2, prepare_json_with, Rules};
use serde::Deserialize;
use serde_json::Value;
use std::{fs, path::Path};

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

pub fn request(family: &Path, id: &str) -> Value {
    serde_json::from_slice(&fs::read(family.join(format!("{id}.request.json"))).unwrap()).unwrap()
}

/// A request prepared with the Go reference's rules.
pub fn prepare(request: &Value) -> Value {
    prepare_json_with(
        &serde_json::to_vec(request).unwrap(),
        "golden",
        Rules::GoReference,
    )
    .unwrap_or_else(|err| panic!("{err}"))
}

pub fn simulate(prepared: Value) -> Value {
    let prepared: PreparedV2 = serde_json::from_value(prepared).unwrap();
    forever_engine::simulate_prepared(&prepared)
        .unwrap_or_else(|err| panic!("{err:?}"))
        .result
}

/// The comparable view of a result, as tools/prepared_v2.py `comparable` writes the goldens:
/// identified lists keyed by ID with an occurrence number, empty containers dropped, no timing
/// or log.
pub fn comparable(result: &Value) -> Value {
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
pub fn differences(go: &Value, rust: &Value, path: &str, out: &mut Vec<String>) {
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

/// Every case of the family's manifest prepared, simulated and compared with its Go result and,
/// where the case keeps one, the first fight's log line by line. Answers each failure.
pub fn check_family(family: &Path) -> Vec<String> {
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(family.join("manifest.json")).unwrap()).unwrap();
    assert!(!manifest.cases.is_empty());
    let mut failures = Vec::new();
    for case in &manifest.cases {
        let result = simulate(prepare(&request(family, &case.id)));
        let golden: Value = serde_json::from_slice(
            &fs::read(family.join(format!("{}.go-result.json", case.id))).unwrap(),
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
            let expected = fs::read_to_string(family.join(log)).unwrap();
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
    failures
}
