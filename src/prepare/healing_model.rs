//! Go health.go: the player's healing model, `trackChanceOfDeath`, `applyHealingModel` and
//! `GetPresimOptions`. Go reads the model only for a player a target swings at, except for the
//! presim: a request with a cadence but no healing measures the damage the player takes in a
//! first run and heals 1.5 times it, tanking or not. The fight runs the heals
//! (`core::fight::healing_model`); preparation registers the shield's aura where Go does and
//! describes the rest as the `healing_model` effect.

use std::panic::{catch_unwind, AssertUnwindSafe};

use serde_json::{json, Value};

use crate::contracts::prepared_v2::ActionId;
use crate::contracts::request::Message;

use super::aura_helpers::AbsorptionAuraConfig;
use super::env::Environment;
use super::sim::AuraConfig;
use super::Refusal;

/// Go `OtherAction_OtherActionHealingModel`, the model's health metrics and shield.
const HEALING_MODEL: &str = "OtherActionHealingModel";

/// Go's `HealingModel` fields Rust reads.
struct Model {
    hps: f64,
    cadence: f64,
    variation: f64,
    absorb_frac: f64,
    burst_window: i32,
}

impl Model {
    fn of(model: &Message) -> Model {
        Model {
            hps: model.f64("hps"),
            cadence: model.f64("cadence_seconds"),
            variation: model.f64("cadence_variation"),
            // Go `Clamp(healingModel.AbsorbFrac, 0, 1)`.
            absorb_frac: model.f64("absorb_frac").clamp(0.0, 1.0),
            burst_window: model.i32("burst_window"),
        }
    }

    /// Go `GetPresimOptions`: a cadence without healing asks for a presim.
    fn presim(&self) -> bool {
        self.hps == 0.0 && self.cadence != 0.0
    }
}

fn player_message(env: &Environment) -> Option<&Message> {
    env.request
        .message("raid")?
        .messages("parties")
        .first()?
        .messages("players")
        .first()
        .copied()
}

fn shield_label(env: &Environment) -> String {
    format!(
        "Healing Model Absorb Shield{}",
        env.sim.unit(env.player).label
    )
}

/// Go `trackChanceOfDeath` after the Chance of Death aura, for the player's request: refuses
/// what Rust does not run, and registers the shield's aura of `applyHealingModel` when the model
/// heals a tank with a share of each heal as a shield.
pub(crate) fn track(env: &mut Environment, player: &Message) -> Result<(), Refusal> {
    let Some(message) = player.message("healing_model") else {
        return Ok(());
    };
    let model = Model::of(message);
    // The Protection Warrior's and Paladin's constructors (`NewProtectionWarrior`,
    // `NewProtectionPaladin`) apply Inspiration at the model's uptime, tanking or not.
    let spec = player.oneof("spec").map(|(name, _)| name);
    if message.f64("inspiration_uptime") > 0.0
        && matches!(spec, Some("protection_warrior" | "protection_paladin"))
    {
        return Err(Refusal::new(
            "healing_model",
            "a healing model's Inspiration uptime is unsupported".to_string(),
        ));
    }
    let tanking = env.tanking();
    let heals = (tanking && model.hps != 0.0) || model.presim();
    if !heals {
        return Ok(());
    }
    if model.cadence < 0.0 || model.variation < 0.0 {
        return Err(Refusal::new(
            "healing_model",
            "a healing model with a negative cadence or cadence variation is unsupported"
                .to_string(),
        ));
    }
    if !model.presim() && model.absorb_frac > 0.0 {
        let label = shield_label(env);
        let unit = env.player;
        env.sim.new_damage_absorption_aura(
            unit,
            AbsorptionAuraConfig {
                aura: AuraConfig {
                    label,
                    action_id: Some(ActionId {
                        other_id: HEALING_MODEL.to_string(),
                        ..ActionId::default()
                    }),
                    ..AuraConfig::default()
                },
                damage_multiplier: 1.0,
                ..AbsorptionAuraConfig::default()
            },
        );
    }
    Ok(())
}

/// The player's auras, inactive at reset, whose activation at one stack or at most changes a
/// healing taken multiplier the model reads, each checked in a reset simulation of its own as
/// `block_value::changing_auras` does. Go reads the multipliers at every heal; the fight reads
/// them at the reset.
fn changing_auras(env: &Environment) -> Vec<String> {
    let fresh = env.fresh();
    let read = |env: &Environment| {
        let pseudo = &env.sim.unit(env.player).pseudo_stats;
        (
            pseudo.healing_taken_multiplier.to_bits(),
            pseudo.external_healing_taken_multiplier.to_bits(),
        )
    };
    let base = read(&fresh);
    let mut changing = Vec::new();
    for aura in &fresh.sim.unit(fresh.player).auras {
        let aura = fresh.sim.aura(*aura);
        if aura.active {
            continue;
        }
        let label = aura.label.clone();
        // A panic while activating an aura counts as a change.
        let changes = catch_unwind(AssertUnwindSafe(|| {
            let mut probe = fresh.clone();
            let Some(aura) = probe.sim.get_aura(probe.player, &label) else {
                return false;
            };
            probe.sim.activate(aura);
            let mut changes = read(&probe) != base;
            let max_stacks = probe.sim.aura(aura).max_stacks;
            if max_stacks > 1 && probe.sim.aura(aura).active {
                probe.sim.set_stacks(aura, max_stacks);
                changes |= read(&probe) != base;
            }
            changes
        }))
        .unwrap_or(true);
        if changes {
            changing.push(label);
        }
    }
    changing
}

/// The `healing_model` effect, after Chance of Death: the heals of a tank's model or a presim,
/// and a tank's TMI window. `None` when the model changes nothing.
pub(crate) fn effect(env: &Environment) -> Result<Option<Value>, Refusal> {
    let Some(message) = player_message(env).and_then(|player| player.message("healing_model"))
    else {
        return Ok(None);
    };
    let model = Model::of(message);
    let tanking = env.tanking();
    let heals = (tanking && model.hps != 0.0) || model.presim();
    // Go `trackChanceOfDeath`: the TMI window of a tank's model.
    let tmi_bin = if tanking { model.burst_window } else { 0 };
    if !heals && tmi_bin == 0 {
        return Ok(None);
    }
    if heals {
        if let Some(label) = changing_auras(env).first() {
            return Err(Refusal::new(
                "healing_model",
                format!("aura {label} changes the healing taken multipliers a healing model reads"),
            ));
        }
    }
    // Go `applyHealingModel`: a zero cadence heals every 2 seconds. The presim's model keeps
    // only the cadence.
    let (hps, median, variation, absorb_frac) = if model.presim() {
        (0.0, model.cadence, 0.0, 0.0)
    } else if heals {
        let median = if model.cadence == 0.0 {
            2.0
        } else {
            model.cadence
        };
        (model.hps, median, model.variation, model.absorb_frac)
    } else {
        (0.0, 2.0, 0.0, 0.0)
    };
    let absorb_aura = (heals && absorb_frac > 0.0).then(|| shield_label(env));
    if let Some(label) = &absorb_aura {
        if env.sim.get_aura(env.player, label).is_none() {
            return Err(Refusal::new(
                "healing_model",
                "the healing model's shield is not registered".to_string(),
            ));
        }
    }
    let pseudo = &env.sim.unit(env.player).pseudo_stats;
    let mut effect = json!({
        "kind": "healing_model",
        "hps": hps,
        "median_cadence": median,
        "cadence_variation": variation,
        "absorb_frac": absorb_frac,
        "healing_taken_multiplier": pseudo.healing_taken_multiplier,
        "external_healing_taken_multiplier": pseudo.external_healing_taken_multiplier,
        "tmi_bin": tmi_bin,
    });
    if let Some(label) = absorb_aura {
        effect["absorb_aura"] = json!(label);
    }
    if model.presim() {
        effect["presim"] = json!(true);
    }
    Ok(Some(effect))
}
