//! The Shaman's shields: Go `sim/shaman` `shields.go`.

use crate::prepare::aura_helpers::{CallbackMask, HitOutcome, ProcTrigger};
use crate::prepare::sim::{AuraConfig, Cooldown, Rules, Sim, UnitId, SECOND};
use crate::prepare::spell::{
    school, CastConfig, DefenseType, ProcMask, SpellConfig, SpellFlag, GCD_DEFAULT,
};

use super::spell_data::spell_data;
use super::spells::{default_cast, flat_cost, spell_action};
use super::{flags, masks, Shaman};

/// Water Shield (408510) sits on the Restoration talent line with no rank subtext, so
/// gen_spelldata makes no table for it and the ids and values are pinned from the client
/// (1.60.1.70532): three globes of 2% maximum mana (408511), one every 3.5 sec at most, no mana
/// cost and, since 70532, no cooldown (CategoryRecoveryTime 15000 to 0); the Go reference keeps
/// the 15 sec cooldown.
pub(super) const WATER_SHIELD_SPELL_ID: i32 = 408510;
pub(super) const WATER_SHIELD_GLOBES: i32 = 3;

/// Totem of Sustaining (23200) under Forever's rules: "Equip: Increases the maximum number of
/// charges on your Water Shield by 1." (Wowhead's Forever tooltip, 18 November 2026). Classic's
/// totem added healing to Lesser Healing Wave; the live one is the charge. Its equip spell,
/// Increased Water Shield Charges (28856, ItemEffect 104176, on equip), is readable in the live
/// client (wago.tools `product=wowf`, build 1.60.1.70959, 18 November 2026):
///
/// - SpellEffect 705031: `A_ADD_FLAT_MODIFIER` (107) of 1 on `SPELLMOD_CHARGES` (misc 4), class
///   mask words 0, 32, 0, 0, the bit Water Shield's row carries.
///
/// So the cast puts up four globes. The Go reference has no row for the item, and only Forever's
/// rules read the client's.
pub(crate) const TOTEM_OF_SUSTAINING: i32 = 23200;

/// The globes a cast of Water Shield puts up for this character: three, and four with Totem of
/// Sustaining worn under Forever's rules. The count is fixed for the fight, which an item swap
/// that takes the totem off or puts it on cannot follow: a request whose swap moves the totem
/// prepares with three, as Rage of the Storm's mod is left out of one that moves the mace.
pub(super) fn water_shield_globes(sim: &Sim, unit: UnitId) -> i32 {
    if sim.rules != Rules::Forever {
        return WATER_SHIELD_GLOBES;
    }
    let worn = sim
        .character(unit)
        .equipment
        .iter()
        .any(|item| item.id == TOTEM_OF_SUSTAINING);
    let moved = sim.item_swap(unit).is_some_and(|swap| {
        swap.slots.iter().any(|&slot| {
            swap.original_equip[slot].id == TOTEM_OF_SUSTAINING
                || swap.swap_equip[slot].id == TOTEM_OF_SUSTAINING
        })
    });
    WATER_SHIELD_GLOBES + i32::from(worn && !moved)
}
pub(super) const WATER_SHIELD_MANA_SPELL_ID: i32 = 408511;
pub(super) const WATER_SHIELD_MANA_PERCENT: f64 = 2.0;
pub(super) const WATER_SHIELD_ICD: crate::prepare::sim::Duration =
    3500 * crate::prepare::sim::MILLISECOND;

/// The trigger by which the shaman's own critical spells spend a globe: MythicSim patch 145 for
/// heals, and since client 70532 harmful spells too (proc flags 156328 to 221864).
pub(super) fn water_shield_crit_trigger() -> ProcTrigger {
    let row = crate::prepare::spelldata::store::must_find(WATER_SHIELD_SPELL_ID);
    ProcTrigger {
        name: "Water Shield Spell Crit Trigger".to_string(),
        callback: CallbackMask(CallbackMask::ON_SPELL_HIT_DEALT.0 | CallbackMask::ON_HEAL_DEALT.0),
        proc_mask: ProcMask(ProcMask::SPELL_DAMAGE.0 | ProcMask::SPELL_HEALING.0),
        outcome: HitOutcome::CRIT,
        can_proc_from_procs: row.can_proc_from_procs(),
        ..ProcTrigger::default()
    }
}

impl Shaman {
    /// Go `registerShieldsSpells`.
    pub(super) fn register_shields_spells(&self, sim: &mut Sim, unit: UnitId) {
        self.register_water_shield_spell(sim, unit);
        self.register_lightning_shield_spell(sim, unit);
        self.register_shield_effect_trigger_spell(sim, unit);
    }

    /// Go `registerShieldEffectTriggerSpell`.
    fn register_shield_effect_trigger_spell(&self, sim: &mut Sim, unit: UnitId) {
        sim.register_spell(
            unit,
            SpellConfig {
                flags: SpellFlag::NO_METRICS | SpellFlag::NO_LOGS,
                class_spell_mask: masks::SHIELD_SELF_PROC,
                ..SpellConfig::default()
            },
        );
    }

    /// Go `registerWaterShieldSpell`.
    fn register_water_shield_spell(&self, sim: &mut Sim, unit: UnitId) {
        if !self.has_talent("water_shield") {
            return;
        }
        let action_id = spell_action(WATER_SHIELD_SPELL_ID);
        // One globe every 3.5 sec at most, whichever trigger spends it: the row has one internal
        // cooldown, a timer of its own.
        let _icd = sim.new_timer(unit);
        let aura = sim.register_aura(
            unit,
            AuraConfig {
                label: "Water Shield".to_string(),
                action_id: Some(action_id.clone()),
                duration: 10 * 60 * SECOND,
                max_stacks: water_shield_globes(sim, unit),
                ..AuraConfig::default()
            },
        );
        sim.attach_proc_trigger(
            aura,
            &ProcTrigger {
                name: "Water Shield Trigger".to_string(),
                callback: CallbackMask::ON_SPELL_HIT_TAKEN,
                class_spell_mask: masks::SHIELD_SELF_PROC,
                ..ProcTrigger::default()
            },
        );
        // MythicSim patch 145: the shaman's own critical spells spend a globe too, as client
        // 70425's row and the 16 October 2026 beta notes state.
        sim.attach_proc_trigger(aura, &water_shield_crit_trigger());

        let cast = if sim.rules == Rules::Forever {
            default_cast(GCD_DEFAULT, 0)
        } else {
            CastConfig {
                cd: Cooldown {
                    timer: Some(sim.new_timer(unit)),
                    duration: 15 * SECOND,
                },
                ..default_cast(GCD_DEFAULT, 0)
            }
        };
        sim.register_spell(
            unit,
            SpellConfig {
                action_id,
                spell_school: school::NATURE,
                defense_type: DefenseType::Magic,
                flags: SpellFlag::APL | flags::INSTANT,
                cast,
                related_self_buff: Some(aura),
                ..SpellConfig::default()
            },
        );
    }

    /// Go `registerLightningShieldSpell`.
    fn register_lightning_shield_spell(&self, sim: &mut Sim, unit: UnitId) {
        let data = spell_data();
        let rank = data.lightning_shield.highest();
        // The shield spell itself carries no damage; the orb that fires is a separate spell, and
        // the rank the shaman knows has its own (rank 7, 26363, at level 60).
        let orb = super::spell_data::lightning_shield_orb();
        let action_id = spell_action(rank.id);

        sim.register_spell(
            unit,
            SpellConfig {
                action_id: spell_action(orb.id),
                spell_school: orb.spell_school(),
                defense_type: orb.defense_type_core(),
                proc_mask: ProcMask::EMPTY,
                flags: flags::SHAMAN_SPELL | SpellFlag::PASSIVE_SPELL,
                class_spell_mask: masks::LIGHTNING_SHIELD,
                damage_multiplier: 1.0,
                threat_multiplier: 1.0,
                bonus_coefficient: orb.damage_effect().coeff(),
                ..SpellConfig::default()
            },
        );

        let aura = sim.register_aura(
            unit,
            AuraConfig {
                label: "Lightning Shield".to_string(),
                action_id: Some(action_id.clone()),
                duration: rank.duration(),
                max_stacks: i32::from(rank.proc_charges),
                ..AuraConfig::default()
            },
        );
        sim.attach_proc_trigger(
            aura,
            &ProcTrigger {
                name: "Lightning Shield Trigger".to_string(),
                callback: CallbackMask::ON_SPELL_HIT_TAKEN,
                icd: 3500 * crate::prepare::sim::MILLISECOND,
                class_spell_mask: masks::SHIELD_SELF_PROC,
                ..ProcTrigger::default()
            },
        );

        sim.register_spell(
            unit,
            SpellConfig {
                action_id,
                spell_school: school::NATURE,
                defense_type: DefenseType::Magic,
                flags: SpellFlag::APL | flags::INSTANT,
                cost: flat_cost(rank.cost() as i32),
                cast: default_cast(rank.gcd(), 0),
                related_self_buff: Some(aura),
                ..SpellConfig::default()
            },
        );
    }
}
