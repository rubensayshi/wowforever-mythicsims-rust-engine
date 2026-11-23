//! Go health.go `applyHealingModel` and metrics_aggregator.go `calculateTMI`: a fixed healer
//! on the player, and the tank's TMI.
//!
//! The model is one pending action from the reset. Each run heals `hps` times the seconds since
//! the last heal times the healing taken multipliers, puts the shield's share on as an absorb
//! shield, then rolls the time to the next heal: a sign roll picks the half below or above the
//! median cadence and a magnitude roll the point in it. The first run, at 0:00, heals nothing
//! and only rolls.
//!
//! TMI keeps every health removal of the iteration as a share of maximum health, sums them over
//! windows of `tmi_bin` seconds that start each whole second, and reports
//! `10 ln(mean(e^(10 window)))` for the iteration.

use crate::contracts::prepared_v2::{ActionId, Effect};
use crate::core::time::{from_seconds, seconds, NS_PER_SECOND};

use super::metrics::Distribution;
use super::{Action, Agent, AuraRef, BuildError, Fight, SpellResult, PRIORITY_GCD};

/// The heals of the model.
#[derive(Clone, Debug)]
pub(crate) struct Heals {
    pub(crate) hps: f64,
    median: f64,
    min_cadence: f64,
    variation_low: f64,
    variation: f64,
    absorb_frac: f64,
    healing_taken_multiplier: f64,
    external_healing_taken_multiplier: f64,
    /// The shield's aura and the strength it has left; zero while it is down, as Go's
    /// `OnExpire` leaves it.
    shield: Option<(AuraRef, f64)>,
    metrics: usize,
    /// Registered after every other reset effect, as Go's presim result registers it.
    pub(crate) late: bool,
    /// Go `timeToNextHeal`.
    time_to_next: i64,
}

/// The model's state in a fight.
#[derive(Clone, Debug, Default)]
pub(crate) struct HealingModel {
    pub(crate) heals: Option<Heals>,
    /// Go `UnitMetrics.tmiBin`; zero keeps TMI at zero.
    tmi_bin: i32,
    /// Go `tmiList`: each health removal's time and its share of maximum health.
    tmi_events: Vec<(i64, f64)>,
    pub(crate) tmi: Distribution,
}

/// Go `calculateTMI` over an iteration of `duration` with its health removals.
pub(crate) fn calculate_tmi(events: &[(i64, f64)], bin: i32, duration: i64) -> f64 {
    if events.is_empty() || bin == 0 {
        return 0.0;
    }
    let bin = i64::from(bin);
    let mut first = 0;
    let mut end = 0;
    let last = events.len();
    let mut buckets: Vec<f64> = Vec::new();
    let mut step: i64 = 0;
    while (step as f64) < seconds(duration) - bin as f64 {
        while first < last && seconds(events[first].0) < step as f64 {
            first += 1;
        }
        while end < last && seconds(events[end].0) < (step + bin) as f64 {
            end += 1;
        }
        if end > first {
            let mut sum = 0.0;
            for event in &events[first..end] {
                sum += event.1;
            }
            buckets.push(sum);
        } else if first < last {
            // A window with no damage in the middle of the fight still counts.
            buckets.push(0.0);
        }
        step += 1;
    }
    if buckets.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    for bucket in &buckets {
        // Go `math.Pow(math.E, x)`, which may differ from Rust's in the last bit.
        sum += std::f64::consts::E.powf(bucket * 10.0);
    }
    10.0 * (1.0 / buckets.len() as f64 * sum).ln()
}

impl<A: Agent> Fight<A> {
    /// The `healing_model` effect: its heals, their health metrics and shield, and TMI.
    pub(crate) fn bind_healing_model(&mut self, effects: &[Effect]) -> Result<(), BuildError> {
        for effect in effects {
            let Effect::HealingModel {
                hps,
                median_cadence,
                cadence_variation,
                absorb_frac,
                absorb_aura,
                healing_taken_multiplier,
                external_healing_taken_multiplier,
                tmi_bin,
                presim,
            } = effect
            else {
                continue;
            };
            if self.healing_model.is_some() {
                return Err("a second healing model".into());
            }
            let heals = if *hps != 0.0 || *presim {
                let shield = match absorb_aura {
                    Some(label) => Some((self.player_aura(label)?, 0.0)),
                    None => None,
                };
                let metrics = self.new_health_metrics(ActionId {
                    other_id: "OtherActionHealingModel".into(),
                    ..ActionId::default()
                });
                let min_cadence = (median_cadence - cadence_variation).max(0.0);
                Some(Heals {
                    hps: *hps,
                    median: *median_cadence,
                    min_cadence,
                    variation_low: median_cadence - min_cadence,
                    variation: *cadence_variation,
                    absorb_frac: *absorb_frac,
                    healing_taken_multiplier: *healing_taken_multiplier,
                    external_healing_taken_multiplier: *external_healing_taken_multiplier,
                    shield,
                    metrics,
                    late: *presim,
                    time_to_next: 0,
                })
            } else {
                None
            };
            self.healing_model = Some(Box::new(HealingModel {
                heals,
                tmi_bin: *tmi_bin,
                tmi_events: Vec::new(),
                tmi: Distribution::default(),
            }));
        }
        Ok(())
    }

    /// Go `Metrics.reset` and the model's reset effect, registered early (`late` false) or after
    /// every other reset effect: the first heal at 0:00.
    pub(crate) fn healing_model_reset(&mut self, late: bool) {
        let Some(model) = self.healing_model.as_deref_mut() else {
            return;
        };
        if !late {
            model.tmi_events.clear();
        }
        let Some(heals) = model.heals.as_mut().filter(|heals| heals.late == late) else {
            return;
        };
        heals.time_to_next = 0;
        if let Some((_, strength)) = heals.shield.as_mut() {
            *strength = 0.0;
        }
        self.schedule(0, PRIORITY_GCD, Action::HealingModel);
    }

    /// The model's pending action: the heal since the last one, then the time to the next.
    pub(crate) fn healing_model_tick(&mut self) {
        let heals = self
            .healing_model
            .as_deref()
            .and_then(|model| model.heals.clone())
            .expect("a scheduled healing model heals");
        let heal = heals.hps
            * (heals.time_to_next as f64 / NS_PER_SECOND as f64)
            * heals.healing_taken_multiplier
            * heals.external_healing_taken_multiplier;
        if heal > 0.0 {
            self.gain_health(heal * (1.0 - heals.absorb_frac), heals.metrics);
            if let Some((aura, strength)) = heals.shield {
                // Go `DamageAbsorptionAura.Activate`: the aura, then the strength its
                // calculator answers, the larger of what is left and the heal's share, and the
                // stacks as whole points.
                self.activate_aura(aura);
                let amount = strength.max(heal * heals.absorb_frac);
                self.set_healing_model_shield(amount);
                let stacks = (amount as i32).max(1);
                self.aura_mut(aura).max_stacks = stacks;
                self.set_stacks(aura, stacks);
            }
        }
        let sign = self.random("Healing Cadence Variation Sign");
        let magnitude = self.random("Healing Cadence Variation Magnitude");
        // The arm64 build fuses both into one multiply-add.
        let next = if sign < 0.5 {
            from_seconds(magnitude.mul_add(heals.variation_low, heals.min_cadence))
        } else {
            from_seconds(magnitude.mul_add(heals.variation, heals.median))
        };
        if let Some(heals) = self
            .healing_model
            .as_deref_mut()
            .and_then(|model| model.heals.as_mut())
        {
            heals.time_to_next = next;
        }
        self.schedule(self.now + next, PRIORITY_GCD, Action::HealingModel);
    }

    fn set_healing_model_shield(&mut self, amount: f64) {
        if let Some((_, strength)) = self
            .healing_model
            .as_deref_mut()
            .and_then(|model| model.heals.as_mut())
            .and_then(|heals| heals.shield.as_mut())
        {
            *strength = amount;
        }
    }

    /// The shield's damage taken modifier, the player's first: it takes what it can of a hit
    /// of any school that deals damage, its stacks follow the strength left, and it fades when
    /// spent.
    pub(crate) fn healing_model_absorb_taken(&mut self, result: &mut SpellResult) {
        let Some((aura, strength)) = self
            .healing_model
            .as_deref()
            .and_then(|model| model.heals.as_ref())
            .and_then(|heals| heals.shield)
        else {
            return;
        };
        if !self.aura(aura).active || result.damage <= 0.0 {
            return;
        }
        let absorbed = strength.min(result.damage);
        result.damage -= absorbed;
        let strength = strength - absorbed;
        self.set_healing_model_shield(strength);
        if self.log.is_some() {
            let line = format!(
                "{} absorbed {absorbed:.1} damage, new shield strength: {strength:.1}",
                self.aura(aura).label
            );
            self.player_log(&line);
        }
        if strength <= 0.0 {
            self.deactivate_aura(aura);
            self.set_healing_model_shield(0.0);
            return;
        }
        self.set_stacks(aura, strength as i32);
    }

    /// Go `RemoveHealth`'s TMI entry for a tank: the time and the share of maximum health.
    #[inline]
    pub(crate) fn tmi_health_removed(&mut self, amount: f64, max_health: f64) {
        if let Some(model) = self.healing_model.as_deref_mut() {
            if model.tmi_bin != 0 {
                model.tmi_events.push((self.now, amount / max_health));
            }
        }
    }

    /// Go `UnitMetrics.doneIteration`'s TMI for a tank with a TMI window.
    pub(crate) fn tmi_done_iteration(&mut self, seed: i64, keep: bool) {
        let duration = self.duration;
        let Some(model) = self.healing_model.as_deref_mut() else {
            return;
        };
        if model.tmi_bin == 0 {
            return;
        }
        // Go multiplies by the duration for the distribution, which divides it out again.
        model.tmi.total =
            calculate_tmi(&model.tmi_events, model.tmi_bin, duration) * seconds(duration);
        model.tmi.done_iteration(duration, seed, keep);
    }

    /// The tank's TMI distribution, when it has a TMI window.
    pub(crate) fn tmi_report(&self) -> Option<super::metrics::DistributionReport> {
        self.healing_model
            .as_deref()
            .filter(|model| model.tmi_bin != 0)
            .map(|model| model.tmi.report())
    }
}

#[cfg(test)]
mod tests {
    use super::calculate_tmi;
    use crate::core::time::NS_PER_SECOND;

    #[test]
    fn tmi_is_zero_without_a_window_or_damage() {
        assert_eq!(calculate_tmi(&[], 6, 60 * NS_PER_SECOND), 0.0);
        assert_eq!(
            calculate_tmi(&[(NS_PER_SECOND, 0.5)], 0, 60 * NS_PER_SECOND),
            0.0
        );
    }

    /// Go's windows start each whole second while the start is below the duration less the
    /// window: a 10 s fight with a 6 s window has windows from 0, 1, 2 and 3 s. A hit of half the
    /// health at 4.5 s is in every window. Hits at 0.5 and 8.5 s are in the first and last; the
    /// two between count zero, since a later hit follows them, and a window after the last hit
    /// would not count.
    #[test]
    fn tmi_sums_each_window() {
        let events = [(4 * NS_PER_SECOND + NS_PER_SECOND / 2, 0.5)];
        let tmi = calculate_tmi(&events, 6, 10 * NS_PER_SECOND);
        assert!((tmi - 50.0).abs() < 1e-12, "{tmi}");
        let events = [
            (NS_PER_SECOND / 2, 0.5),
            (8 * NS_PER_SECOND + NS_PER_SECOND / 2, 0.5),
        ];
        let tmi = calculate_tmi(&events, 6, 10 * NS_PER_SECOND);
        let expected = 10.0 * ((2.0 * 5f64.exp() + 2.0) / 4.0).ln();
        assert!((tmi - expected).abs() < 1e-12, "{tmi} {expected}");
        let events = [(NS_PER_SECOND / 2, 0.5)];
        let tmi = calculate_tmi(&events, 6, 10 * NS_PER_SECOND);
        assert!((tmi - 50.0).abs() < 1e-12, "{tmi}");
    }
}
