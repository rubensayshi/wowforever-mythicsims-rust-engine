//! Volley (1510, 14294, 14295), from Go sim/hunter/volley.go: the channel holds the ranged
//! swing for the rank's duration, stops the melee swing until it ends (a channel is a non-melee
//! cast, and the swing restarts a full swing after it) and applies its area dot on the hunter, which snapshots the
//! Go literal tick of the rank plus, under Forever's rules, a share of ranged attack power that
//! each tick reads again from the hunter's current ranged attack power. Every tick is its own ranged damage spell, as the client row
//! has a ranged defense type and no Cannot Crit, so it rolls the ranged hit and crit tables
//! on each target, in unit index order, as Multi-Shot does.

use crate::core::fight::{melee::PhysicalOutcome, Agent, DotId, Fight, Outcome, Side, SpellId};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Volley {
    tick_base: f64,
    attack_power_share: f64,
    ranged_delay: i64,
}

pub(crate) fn bind<A: Agent>(
    fight: &mut Fight<A>,
    spell: SpellId,
    tick_base: f64,
    attack_power_share: f64,
    ranged_delay: i64,
) -> Result<Volley, String> {
    let dot = fight.spells[spell].dot.ok_or("Volley has no channel")?;
    fight.dots[dot].tick_base = Some(tick_base);
    Ok(Volley {
        tick_base,
        attack_power_share,
        ranged_delay,
    })
}

impl Volley {
    /// The cast's `ApplyEffects`: `DelayRangedUntil`, `StopMeleeUntil`, then `AOEDot().Apply`.
    pub(crate) fn apply<A: Agent>(&self, fight: &mut Fight<A>, spell: SpellId) {
        fight.delay_ranged_until(fight.now + self.ranged_delay);
        fight.stop_melee_until(fight.now + self.ranged_delay);
        let dot = fight.spells[spell].dot.expect("Volley has a channel");
        if self.attack_power_share == 0.0 {
            fight.apply_dot(dot);
            return;
        }
        // As Serpent Sting: the snapshot holds the share of the cast's ranged attack power, and
        // each tick swaps it for the share of the ranged attack power it has then.
        let attack_power = fight.ranged_attack_power();
        fight.dots[dot].tick_base = Some(
            self.attack_power_share
                .mul_add(attack_power, self.tick_base),
        );
        fight.apply_dot(dot);
        fight.snapshot_dot_attack_power_share(dot, self.attack_power_share, true);
    }

    /// The area dot's `OnTick`.
    pub(crate) fn tick<A: Agent>(&self, fight: &mut Fight<A>, dot: DotId) {
        let spell = fight.dots[dot].spell;
        for position in 0..fight.targets.len() {
            let result = fight.snapshot_dot_tick_calc_with(
                dot,
                Side::target(position),
                Outcome::Table(PhysicalOutcome::RangedHitAndCrit { count: true }),
            );
            fight.deal_damage(spell, result, true);
        }
    }
}
