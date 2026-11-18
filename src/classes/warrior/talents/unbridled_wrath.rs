//! Unbridled Wrath (12322, rage from 12964), from Go sim/warrior/talents_fury.go
//! `registerUnbridledWrath`: a proc trigger on landed white hits that dealt damage, which
//! grants rage a spell batch window later. Every weapon gives the same rage: the client
//! hotfix 112347 dropped the doubling for a two-handed weapon. Heroic Strike and Cleave take a
//! main hand swing's place but carry the melee special mask, which the trigger excludes
//! (community #692). Under Forever's rules since the hotfixes of 16 November 2026 ("Fixed an
//! issue where Unbridled Wrath was not activated by Cleave or Heroic Strike"; the row's
//! ProcFlags went from 0x4 to 0x14) those two count as well; no other melee ability does.

use crate::core::fight::{Agent, AuraRef, Fight, SpellId, SpellResult};

#[derive(Clone, Copy, Debug)]
pub(crate) struct UnbridledWrath {
    pub(crate) trigger: AuraRef,
    pub(crate) proc_chance: f64,
    pub(crate) rage: f64,
    /// Go `NewRageMetrics(actionID)`.
    pub(crate) metrics: usize,
    /// Forever's rules since the hotfixes of 16 November 2026: Heroic Strike and Cleave proc it.
    pub(crate) strikes_proc: bool,
}

/// The trigger's OnSpellHitDealt.
pub(crate) fn on_hit<A: Agent>(
    fight: &mut Fight<A>,
    params: UnbridledWrath,
    spell: SpellId,
    result: &SpellResult,
) {
    let state = &fight.spells[spell];
    let strike = params.strikes_proc
        && matches!(
            state.class_spell.as_deref(),
            Some("heroic_strike" | "cleave")
        );
    if state.flags.proc
        || !state.white_hit
        || (state.melee_special && !strike)
        || !result.landed()
        || result.damage == 0.0
    {
        return;
    }
    if params.proc_chance != 1.0 && fight.random_for_aura(params.trigger) > params.proc_chance {
        return;
    }
    fight.schedule_delayed_proc(params.trigger, spell, *result);
}

/// The delayed handler.
pub(crate) fn grant<A: Agent>(fight: &mut Fight<A>, params: UnbridledWrath) {
    fight.add_rage(params.rage, params.metrics);
}
