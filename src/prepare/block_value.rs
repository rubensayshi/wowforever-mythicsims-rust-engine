//! The player's block value off the target's threat. A tank's is in the rolls of the target's
//! swing, which preparation reads under every stat aura combination (`enemy`). A player nothing
//! swings at has no such rolls, and a class spell that adds the block value to its hit, as a
//! Warrior's Shield Slam does, reads it from its own effect instead: the value under each stat
//! aura combination, read from the same reset simulations as the stat auras effect's, and the
//! auras outside the stat auras whose activation would change it.

use std::panic::{catch_unwind, AssertUnwindSafe};

use super::enemy::{player_block_value, set_stat_auras};
use super::env::Environment;
use super::stat_auras::StatLayout;
use super::Refusal;

/// What a blocked swing takes off and the spell adds, as the simulation stands.
fn reduction(env: &Environment) -> f64 {
    let (value, multiplier) = player_block_value(env);
    value * multiplier
}

/// One pass over the stat aura combinations, as `enemy::EnemyCombos` makes for a tank.
pub(crate) struct BlockCombos {
    request: crate::contracts::request::Message,
    factory: super::env::AgentFactory,
    rules: super::sim::Rules,
    raid_live: super::env::RaidLive,
    layout: StatLayout,
    /// The block value by combination.
    pub reductions: Vec<f64>,
}

impl BlockCombos {
    pub(crate) fn new(env: &Environment, layout: &StatLayout) -> BlockCombos {
        BlockCombos {
            request: env.request.clone(),
            factory: env.factory,
            rules: env.rules,
            raid_live: env.raid_live,
            layout: layout.clone(),
            reductions: Vec::new(),
        }
    }

    /// Reads one combination: the stat auras effect's simulation, or when its own setup left a
    /// different state than `set_stat_auras` does, one set up here.
    pub(crate) fn read(
        &mut self,
        mask: usize,
        env: &mut Environment,
        exact: bool,
    ) -> Result<(), Refusal> {
        if exact {
            self.reductions.push(reduction(env));
            return Ok(());
        }
        let mut fresh =
            Environment::with_options(&self.request, self.factory, self.rules, self.raid_live)?;
        set_stat_auras(&mut fresh, &self.layout, mask);
        self.reductions.push(reduction(&fresh));
        Ok(())
    }
}

/// The block value at reset, for a player without stat auras.
pub(crate) fn at_reset(env: &Environment) -> f64 {
    reduction(&env.fresh())
}

/// The player's auras, inactive at reset and outside the stat auras, whose activation at one
/// stack or at most changes the block value. Each is checked in a reset simulation of its own,
/// as `enemy::enemy_at_reset` checks a tank's swing.
pub(crate) fn changing_auras(env: &Environment, stat_auras: &[String]) -> Vec<String> {
    let fresh = env.fresh();
    let base = reduction(&fresh);
    let mut changing = Vec::new();
    for aura in &fresh.sim.unit(fresh.player).auras {
        let aura = fresh.sim.aura(*aura);
        if aura.active || stat_auras.contains(&aura.label) {
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
            let mut changes = reduction(&probe).to_bits() != base.to_bits();
            // An exclusive category can refuse the activation; such an aura changes nothing.
            let max_stacks = probe.sim.aura(aura).max_stacks;
            if max_stacks > 1 && probe.sim.aura(aura).active {
                probe.sim.set_stacks(aura, max_stacks);
                changes |= reduction(&probe).to_bits() != base.to_bits();
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
