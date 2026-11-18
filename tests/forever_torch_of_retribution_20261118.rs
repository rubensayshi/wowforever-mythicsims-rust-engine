//! Torch of Retribution (10515), a quest item a character holds in both hands, refused a level 55
//! Paladin's Droptimizer on 17 November 2026 ("paladin cannot equip Torch of Retribution", the
//! application's gear check). The engine did not agree: the reference row is a weapon of speed 0.5
//! that hits for 0 to 1, so a Quick Sim, which skips that check, swung it. In the live client it
//! is an item of the armor class (Item row: class 4, subclass 0, two-hand slot, wago.tools
//! `product=wowf`, build 1.60.1.70959) whose Wowhead Forever tooltip of 18 November 2026 states no
//! damage and no speed, the one reference weapon row that is no weapon in the client. A seal lands
//! four times as often at 0.5 as on a real two-hander: the Retribution Paladin board request
//! holding the torch read 1,351 DPS against 996 with its reference weapon.
//!
//! Forever's rules refuse it by name; the reference keeps its row.

use forever_engine::{prepare_json_with, PrepareError, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

const TORCH_OF_RETRIBUTION: i64 = 10515;

const MAIN_HAND: usize = 14;
const OFF_HAND: usize = 15;

/// A fixture's request (a path under `fixtures/`) holding the torch, with no off hand and no item
/// swap.
fn holding_the_torch(fixture: &str) -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("fixtures/{fixture}.request.json"));
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(20);
    let player = &mut request["raid"]["parties"][0]["players"][0];
    let fields = player.as_object_mut().unwrap();
    fields.remove("itemSwap");
    fields.insert("enableItemSwap".to_string(), json!(false));
    let equipment = player["equipment"]["items"].as_array_mut().unwrap();
    equipment.resize(17, json!({}));
    equipment[MAIN_HAND] = json!({ "id": TORCH_OF_RETRIBUTION });
    equipment[OFF_HAND] = json!({});
    request
}

/// Forever's rules refuse the torch by name, in words a player can act on.
#[test]
fn the_torch_of_retribution_is_refused_as_no_weapon() {
    let request = holding_the_torch("paladin/retribution-paladin-unsealed-wrath-command-echoes");
    match prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever) {
        Err(PrepareError::Refused(refusal)) => {
            let text = refusal.to_string();
            assert!(
                text.contains("Torch of Retribution is a quest item, not a weapon"),
                "{text}"
            );
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The reference still prepares its row, a weapon of speed 0.5.
#[test]
fn the_reference_keeps_the_torch_of_retribution() {
    let request = holding_the_torch("forever-gaps/elemental-shaman-the-elements");
    let prepared = prepare_json_with(request.to_string().as_bytes(), "rust", Rules::GoReference)
        .unwrap_or_else(|err| panic!("{err}"));
    assert_eq!(prepared["melee"]["main_hand"]["swing_speed"], 0.5);
}
