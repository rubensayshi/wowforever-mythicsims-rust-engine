//! Bane of Agony (11713), from Go sim/warlock/agony.go. Forever moved Curse of Agony onto
//! the bane slot, which it takes from Bane of Doom. The dot ramps by the time since it landed, in thirds of
//! its duration: half the tick, then the whole tick, then one and a half. Forever combat logs
//! show the extra ticks of a Wrack channel take the share of the third they land in, so the
//! ramp follows time, not the tick count. Amplify Curse raises the whole tick,
//! the ramping base and the spell power share alike (Go `AmplifyCurseWholeTick`, patch 177):
//! the factor the snapshot spends is kept for the target and applied to each tick's damage.

use crate::{
    classes::warlock::agent::WarlockAgent,
    core::fight::{AuraRef, DotId, Fight, Outcome, Side, SpellId},
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct BaneOfAgony {
    pub(crate) dot: DotId,
    tick_base: f64,
    ramp_share: f64,
    /// Go's `rampStep`, the share of the tick the last snapshot stored.
    ramp_step: f64,
    /// Amplify Curse's aura and the factor its snapshot spends it for.
    pub(crate) amplify: Option<(AuraRef, f64)>,
}

impl BaneOfAgony {
    pub(crate) fn new(dot: DotId, tick_base: f64, ramp_share: f64) -> Self {
        Self {
            dot,
            tick_base,
            ramp_share,
            ramp_step: 0.0,
            amplify: None,
        }
    }
}

fn state(fight: &Fight<WarlockAgent>) -> BaneOfAgony {
    fight.agent.bane_of_agony.expect("Bane of Agony is bound")
}

/// `ApplyEffects`: a hit roll without a hit counter, the dot when it lands, then the empty
/// outcome. `Dot.Apply` deactivates a running copy first, whose last tick may still be due
/// and reads the old ramp step; then `OnSnapshot` spends Amplify Curse while it is active,
/// keeping its factor for this target's ticks, stores the new step and snapshots it.
pub(crate) fn apply(fight: &mut Fight<WarlockAgent>, spell: SpellId, target: Side) {
    let result = fight.calc_outcome(spell, target, Outcome::MagicHitNoHitCounter);
    if result.landed() {
        let mut agony = state(fight);
        let dot = fight.dot_on(agony.dot, target);
        let aura = fight.dots[dot].aura;
        super::take_bane_slot(fight, aura);
        fight.deactivate_aura(aura);
        let base = agony.tick_base;
        let mut whole_tick_amp = 1.0;
        if let Some((amplify, factor)) = agony.amplify {
            if fight.aura(amplify).active {
                whole_tick_amp = factor;
                fight.deactivate_aura(amplify);
            }
        }
        let amps = &mut fight.agent.agony_whole_tick_amp;
        if amps.len() <= dot {
            amps.resize(dot + 1, 1.0);
        }
        amps[dot] = whole_tick_amp;
        agony.ramp_step = base * agony.ramp_share;
        fight.agent.bane_of_agony = Some(agony);
        fight.dots[dot].tick_base = Some(agony.ramp_step);
        fight.apply_dot(dot);
        let levels = &mut fight.agent.agony_ramp_level;
        if levels.len() <= dot {
            levels.resize(dot + 1, 0);
        }
        levels[dot] = 0;
    }
    fight.deal_damage(spell, result, false);
}

/// `OnTick`: the stored amount moved to the ramp third this tick lands in, then the snapshot
/// tick raised by the Amplify Curse factor its snapshot spent.
pub(crate) fn tick(fight: &mut Fight<WarlockAgent>, dot: DotId) {
    let agony = state(fight);
    // The ramp follows the time since the curse landed, in thirds of its duration, a tick on a
    // third's end taking that third's share: half the base, then the base, then one and a half.
    // Wrack's added ticks deal the share of the third they land in.
    let aura = fight.dots[dot].aura;
    if fight.aura(aura).active {
        let duration = fight.aura(aura).duration;
        let elapsed = duration - (fight.aura(aura).expires - fight.now);
        let level: i32 = if 3 * elapsed <= duration {
            0
        } else if 3 * elapsed <= 2 * duration {
            1
        } else {
            2
        };
        let levels = &mut fight.agent.agony_ramp_level;
        if levels.len() <= dot {
            levels.resize(dot + 1, 0);
        }
        let delta = level - levels[dot];
        levels[dot] = level;
        fight.dots[dot].snapshot_base += f64::from(delta) * agony.ramp_step;
    }
    let mut result = fight.snapshot_dot_tick_calc(dot);
    let amp = fight
        .agent
        .agony_whole_tick_amp
        .get(dot)
        .copied()
        .unwrap_or(1.0);
    let spell = fight.dots[dot].spell;
    fight.scale_damage(spell, &mut result, amp);
    fight.deal_damage(spell, result, true);
}
