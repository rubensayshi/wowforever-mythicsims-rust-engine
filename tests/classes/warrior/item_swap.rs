//! A Warrior's Dense Sharpening Stone with an item swap set (MythicSim Rust-only). The stone's
//! +8 weapon damage stays on the main hand: the weapon worn at the pull carries it from the start,
//! as it does without a swap set, and a weapon the swap puts in the hand carries it too.
//!
//! Before the fix the swap's reset rebuilt the main hand from its item and left the stone off, so
//! the Arms board reference with an Arcanite Reaper swap set that no rotation line uses simmed
//! 867.21 DPS on bundle-a9551c6 against 877.44 without the set.
//!
//! The request is the Arms board's reference (one target, the board seed), as the worker builds it.

use forever_engine::{contracts::prepared_v2::PreparedV2, prepare_json_with, Rules};
use serde_json::{json, Value};
use std::{fs, path::Path};

const DENSE_SHARPENING_STONE: i64 = 16138;
/// Arcanite Reaper: 153 to 256 at 3.80.
const ARCANITE_REAPER: i64 = 12784;
const REAPER_DAMAGE: (f64, f64) = (153.0, 256.0);

fn board_request() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/classes/warrior/item_swap/arms-board.request.json");
    let mut request: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    request["simOptions"]["iterations"] = json!(200);
    request
}

fn player(request: &mut Value) -> &mut Value {
    &mut request["raid"]["parties"][0]["players"][0]
}

/// The board's request with an Arcanite Reaper swap set, and a first rotation line that swaps
/// to it when `swaps` is set.
fn with_reaper(swaps: bool) -> Value {
    let mut request = board_request();
    let player = player(&mut request);
    let mut items = vec![json!({}); 17];
    items[14] = json!({"id": ARCANITE_REAPER});
    player["enableItemSwap"] = json!(true);
    player["itemSwap"] = json!({ "items": items });
    if swaps {
        let line = json!({"action": {
            "condition": {"activeItemSwapSet": {"swapSet": "Main"}},
            "itemSwap": {"swapSet": "Swap1"},
        }});
        player["rotation"]["priorityList"]
            .as_array_mut()
            .unwrap()
            .insert(0, line);
    }
    request
}

fn without_stone(mut request: Value) -> Value {
    player(&mut request)["consumables"]
        .as_object_mut()
        .unwrap()
        .remove("mhImbueId");
    request
}

fn prepared(request: &Value) -> PreparedV2 {
    let value = prepare_json_with(request.to_string().as_bytes(), "rust", Rules::Forever)
        .unwrap_or_else(|err| panic!("{err}"));
    let prepared: PreparedV2 = serde_json::from_value(value).unwrap();
    assert_eq!(
        forever_engine::prepared_coverage(&prepared),
        Vec::<String>::new()
    );
    prepared
}

fn main_hand(prepared: &PreparedV2) -> (f64, f64) {
    let weapon = &prepared.melee.main_hand;
    (weapon.base_damage_min, weapon.base_damage_max)
}

/// The swap's change to a main hand damage field, which is the swapped weapon's.
fn swapped_main_hand(prepared: &PreparedV2) -> (f64, f64) {
    let value = serde_json::to_value(prepared).unwrap();
    let swap = value["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|effect| effect["kind"] == "item_swap")
        .expect("an item swap");
    let field = |name: &str| {
        swap["swapped"]
            .as_array()
            .unwrap()
            .iter()
            .find(|change| change["path"] == format!("/melee/main_hand/{name}"))
            .unwrap_or_else(|| panic!("no swapped {name}"))["value"]
            .as_f64()
            .unwrap()
    };
    (field("base_damage_min"), field("base_damage_max"))
}

fn dps(prepared: &PreparedV2) -> f64 {
    let report = forever_engine::simulate_prepared(prepared).unwrap();
    report.result["raidMetrics"]["parties"][0]["players"][0]["dps"]["avg"]
        .as_f64()
        .unwrap()
}

#[test]
fn the_stone_is_on_the_worn_main_hand_with_a_swap_set() {
    let request = board_request();
    assert_eq!(
        request["raid"]["parties"][0]["players"][0]["consumables"]["mhImbueId"],
        json!(DENSE_SHARPENING_STONE)
    );
    let plain = prepared(&request);
    let bare = prepared(&without_stone(board_request()));
    let stone = (
        main_hand(&plain).0 - main_hand(&bare).0,
        main_hand(&plain).1 - main_hand(&bare).1,
    );
    assert_eq!(stone, (8.0, 8.0));
    for swaps in [false, true] {
        let with_set = prepared(&with_reaper(swaps));
        assert_eq!(main_hand(&with_set), main_hand(&plain), "swaps: {swaps}");
    }
}

#[test]
fn the_stone_follows_the_weapon_the_swap_puts_in_the_hand() {
    let with_stone = prepared(&with_reaper(true));
    assert_eq!(
        swapped_main_hand(&with_stone),
        (REAPER_DAMAGE.0 + 8.0, REAPER_DAMAGE.1 + 8.0)
    );
    let bare = prepared(&without_stone(with_reaper(true)));
    assert_eq!(swapped_main_hand(&bare), REAPER_DAMAGE);
}

#[test]
fn an_unused_swap_set_sims_as_no_set() {
    let plain = dps(&prepared(&board_request()));
    let unused = dps(&prepared(&with_reaper(false)));
    assert_eq!(unused, plain);
}
