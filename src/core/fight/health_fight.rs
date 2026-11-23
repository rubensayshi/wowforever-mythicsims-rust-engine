//! Go sim.go and target.go for a fight that ends on health (`Encounter.useHealth`): the fight
//! ends once the damage dealt to the targets exceeds the sum of their health, with no time
//! limit, and the execute phases follow the share of that health dealt instead of the clock.
//! The fight's length is a guess until the engine's presim measures it; while it is one, the
//! first iteration runs on ten minutes and later ones on the first's length.

use crate::core::time::{seconds, NS_PER_SECOND};

use super::{Agent, Fight};

/// The health a fight ends at, and the state Go keeps for it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HealthFight {
    /// Go `Encounter.EndFightAtHealth`.
    pub(crate) end_damage: f64,
    /// Go `Encounter.DurationIsEstimate`.
    pub(crate) estimate: bool,
    /// Go `nextExecuteDamage`: the damage taken that starts the next execute phase.
    pub(crate) next_execute_damage: f64,
}

impl<A: Agent> Fight<A> {
    /// Go `Simulation.reset`'s first step: an estimated length becomes the last iteration's.
    pub(crate) fn health_fight_reset(&mut self) {
        if let Some(health) = self.health_fight.as_mut() {
            if health.estimate && self.now != 0 {
                self.config.base_duration = self.now;
                health.estimate = false;
            }
        }
    }

    /// Whether the damage dealt has ended the fight: Go `DamageTaken > endOfCombatDamage`.
    #[inline]
    pub(crate) fn health_fight_over(&self) -> bool {
        self.health_fight
            .is_some_and(|health| self.encounter_damage_taken > health.end_damage)
    }

    /// Go `nextExecutePhase`'s `setup` for a health fight: the phase at `damage` of the health
    /// left. Answers false for a fight timed by duration.
    pub(crate) fn health_execute_phase(&mut self, damage: f64) -> bool {
        let Some(health) = self.health_fight.as_mut() else {
            return false;
        };
        health.next_execute_damage = (1.0 - damage) * health.end_damage;
        true
    }

    /// Whether the damage dealt has reached the next execute phase of a health fight.
    #[inline]
    pub(crate) fn health_execute_due(&self) -> bool {
        self.health_fight
            .is_some_and(|health| self.encounter_damage_taken >= health.next_execute_damage)
    }

    /// Go `GetRemainingDuration`. While a health fight's length is a guess, after 5 seconds it
    /// is the health left at the damage per second so far, in whole seconds.
    pub(crate) fn remaining_duration(&self) -> i64 {
        if let Some(health) = self.health_fight {
            if health.estimate && self.now >= 5 * NS_PER_SECOND {
                let dps = self.encounter_damage_taken / seconds(self.now);
                let left = ((health.end_damage - self.encounter_damage_taken) / dps) as i64;
                return left.wrapping_mul(NS_PER_SECOND);
            }
        }
        self.duration - self.now
    }

    /// Whether the remaining duration may jump rather than fall with the clock: a health
    /// fight's guessed length.
    pub(crate) fn remaining_duration_estimated(&self) -> bool {
        self.health_fight.is_some_and(|health| health.estimate)
    }

    /// Go `GetRemainingDurationPercent`: the share of the targets' health left, or of the time.
    pub(crate) fn remaining_duration_percent(&self) -> f64 {
        match self.health_fight {
            Some(health) => 1.0 - self.encounter_damage_taken / health.end_damage,
            None => (self.duration - self.now) as f64 / self.duration as f64,
        }
    }
}
