//! Wrack (1316697): a shadow channel whose ticks Soul Siphon scales.
//!
//! Under Forever's rules since the class tuning of 11 November 2026, while the channel runs the
//! warlock's Siphon Life, Corruption and Bane of Agony on the target "deal their periodic damage
//! 100% faster" (`dot_rate` 2). Forever combat logs settle how: each dot keeps the tick grid it
//! landed on, and while the channel is on its target the half points between its ticks tick
//! too, each a full tick. Starting or ending the channel moves no tick, and a dot still runs out
//! on a full tick at its expiry. Two public reports on forever.warcraftlogs.com
//! (GbPa1DmnMKT7fqY9, jnm1dPFg7avAJ3tQ; 31 fights, 9,816 dot ticks): every tick after a
//! channel started sat on the dot's half grid (3 of 4,422 off by more than 0.08 s), and
//! the 1,916 half point ticks all fell while Wrack was on the target.
//!
//! The Go reference, from sim/warlock/wrack.go, is the capstone as it was before: while it runs
//! the warlock's Corruption and Bane of Agony on the target take a dynamic damage taken bonus
//! (`dot_bonus`). Neither has a direct hit, so the bonus reaches only ticks.

use crate::core::fight::{Agent, DotId, Fight, SpellDamageTakenModifier};

#[derive(Clone, Debug)]
pub(crate) struct Wrack {
    pub(crate) dot: DotId,
    pub(crate) soul_siphon: f64,
    /// The first target's dots the channel speeds up, and the rate they tick at under it.
    pub(crate) rated_dots: Vec<DotId>,
    pub(crate) dot_rate: f64,
}

/// Resolve the channel's dot and register the bonus on the target, after Improved Shadow
/// Bolt's, as Go registers talents before spells.
pub(crate) fn bind<A: Agent>(
    fight: &mut Fight<A>,
    dot: DotId,
    soul_siphon: f64,
    dot_spells: &[usize],
    dot_bonus: f64,
    dot_rate: f64,
) -> Wrack {
    if dot_bonus != 1.0 {
        let mut spells = vec![false; fight.spells.len()];
        for &spell in dot_spells {
            spells[spell] = true;
        }
        let aura = fight.dots[dot].aura;
        fight
            .spell_damage_taken_modifiers
            .push(SpellDamageTakenModifier {
                spells,
                auras: vec![aura],
                multiplier: dot_bonus,
            });
    }
    let rated_dots = if dot_rate == 1.0 {
        Vec::new()
    } else {
        dot_spells
            .iter()
            .filter_map(|&spell| fight.spell_dot(spell))
            .collect()
    };
    Wrack {
        dot,
        soul_siphon,
        rated_dots,
        dot_rate,
    }
}

/// `OnTick` of a target's dot: the snapshot damage, Soul Siphon's `soul_siphon`, then the tick
/// dealt.
pub(crate) fn tick<A: Agent>(fight: &mut Fight<A>, dot: DotId, soul_siphon: f64) {
    let mut result = fight.snapshot_dot_tick_calc(dot);
    let spell = fight.dots[dot].spell;
    fight.scale_damage(spell, &mut result, soul_siphon);
    fight.deal_damage(spell, result, true);
}

/// The channel starts (`rate` the channel's) or ends (`rate` 1) on the target `channel` is on:
/// the warlock's dots there take that rate.
pub(crate) fn set_dot_rate<A: Agent>(
    fight: &mut Fight<A>,
    channel: DotId,
    rated_dots: &[DotId],
    rate: f64,
) {
    let target = fight.dots[channel].side;
    for &dot in rated_dots {
        let dot = fight.dot_on(dot, target);
        fight.set_dot_tick_rate(dot, rate);
    }
}
