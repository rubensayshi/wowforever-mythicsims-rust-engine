//! Go sim/warrior/talents.go: the Warrior's talents, registered in Go's order.

mod arms;
mod fury;
mod protection;

use crate::prepare::sim::{Duration, MILLISECOND};

/// `time.Millisecond * time.Duration(value)`: the float is cut to whole milliseconds.
pub(super) fn millis(value: f64) -> Duration {
    (value as i64).wrapping_mul(MILLISECOND)
}

/// Gore Drinker's buff label, its spell (1323969) and duration (SpellMisc 869652's SpellDuration
/// 31, 8 sec) on wago.tools' live rows of build 1.60.1.70753; the pinned store lacks the row.
pub(in super::super) const GORE_DRINKER: &str = "Gore Drinker";
pub(in super::super) const GORE_DRINKER_BUFF: i32 = 1323969;
/// Gore Drinker's heal (1323968, a Heal effect), whose health metrics the warrior's gains go
/// under.
pub(in super::super) const GORE_DRINKER_HEAL: i32 = 1323968;
pub(super) const GORE_DRINKER_DURATION: i64 = 8 * crate::prepare::sim::SECOND;
