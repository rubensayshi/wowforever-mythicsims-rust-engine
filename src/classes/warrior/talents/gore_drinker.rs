//! Gore Drinker (Fury, Forever's layout `gore_drinker`, 2 ranks), Forever's rules only. The
//! reference never simulates it, so there is no Go version; the numbers are the client's.
//!
//! - The talent, 1323967 (`data/spells.jsonl`, client 1.60.1.70425): two Dummy effects whose rank
//!   curves (`data/spell-extras.json`) are 3 and 3 melee attacks and 0.5 and 1% of maximum
//!   health, as foreverchanges.pro's text at 2/2 says: "Your Enrage, Berserker Rage, Bloodrage,
//!   Death Wish, Reckless Swings, and Bloodthirst abilities cause your next 3 melee attacks to
//!   restore 1.0% of your maximum Health."
//! - Its buff, 1323969, and heal, 1323968, which the pinned store lacks, from wago.tools
//!   `product=wowf` (build 1.60.1.70753, 23 November 2026): SpellEffect 1360683 is a Proc
//!   Trigger Spell With Value (aura 231) that casts 1323968, whose SpellEffect 1360682 is a Heal
//!   (effect 10); SpellAuraOptions 248613 gives 1323969 3 charges at a 100% chance on
//!   ProcTypeMask 20 (a melee auto attack or a melee ability done), and SpellMisc 869652 a
//!   duration of 8 sec (SpellDuration 31).
//!
//! What the rows leave to the server, and so are assumptions: each landed melee hit of the
//! warrior, white or special and either hand, spends a charge and heals at once, as any proc on a
//! done melee hit lands only on a hit, for the share of the maximum health at that moment times
//! the healing taken multiplier, as a Heal effect reads it; a new trigger puts the buff back at
//! its full 3 charges and 8 sec, as an aura that does not stack does; the triggering
//! Bloodthirst's own hit spends none, since the buff comes from that hit, and a Bloodthirst that
//! misses gives none; Enrage triggers it when the warrior gains it.

use crate::core::fight::{Agent, AuraRef, Fight, SpellId, SpellResult};

/// Gore Drinker's buff and heal (1323969, 1323968).
#[derive(Clone, Copy, Debug)]
pub(crate) struct GoreDrinker {
    pub(crate) aura: AuraRef,
    pub(crate) charges: i32,
    /// The share of maximum health each charge restores.
    pub(crate) health_fraction: f64,
    /// The warrior's healing taken multiplier at the reset, which a heal reads.
    pub(crate) healing_taken_multiplier: f64,
    /// Health metrics of the heal, 1323968.
    pub(crate) metrics: usize,
}

/// A triggering ability or Enrage: the buff at full charges.
pub(crate) fn trigger<A: Agent>(fight: &mut Fight<A>, params: GoreDrinker) {
    fight.activate_aura(params.aura);
    fight.set_stacks(params.aura, params.charges);
}

/// The trigger's OnSpellHitDealt: a landed melee hit while the buff is up spends a charge and
/// heals the warrior.
pub(crate) fn on_hit<A: Agent>(
    fight: &mut Fight<A>,
    params: GoreDrinker,
    spell: SpellId,
    result: &SpellResult,
) {
    let state = &fight.spells[spell];
    if !(state.white_hit || state.melee_special)
        || !result.landed()
        || !fight.aura(params.aura).active
    {
        return;
    }
    let amount =
        fight.player_max_health() * params.health_fraction * params.healing_taken_multiplier;
    fight.remove_stack(params.aura);
    fight.gain_health(amount, params.metrics);
}
