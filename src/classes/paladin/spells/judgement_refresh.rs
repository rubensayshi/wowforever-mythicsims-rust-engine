//! Judgement Refresh, from Go sim/paladin/judgement.go: every melee strike that lands
//! refreshes the active judgement debuffs on its target. Under Forever's rules only a white
//! swing or Windfury Totem's weapon strike does: combat logs of a Retribution paladin show a
//! refresh after 396 of 404 lone white swings and 13 of 13 totem strikes, and after none of
//! 173 Holy Strikes, 69 Judgements of Command, 75 Judgements of Righteousness, 23 Judgements
//! of Fury or 2 Hammers of Wrath. Sacred Arbiter's Holy Strike refresh is its own listener.

use crate::core::fight::{Agent, AuraRef, Fight, Side, SpellId, SpellResult, OUTCOME_LANDED};

#[derive(Clone, Debug)]
pub(crate) struct JudgementRefresh {
    judgements: Vec<AuraRef>,
    /// Forever's rules: only white swings and Windfury Totem's strikes refresh.
    pub(crate) white_swings_only: bool,
}

pub(crate) fn bind<A: Agent>(
    fight: &Fight<A>,
    judgements: &[String],
) -> Result<JudgementRefresh, String> {
    let judgements = judgements
        .iter()
        .map(|label| {
            fight.trackers[Side::Target.index()]
                .find(label)
                .map(|index| AuraRef {
                    side: Side::Target,
                    index,
                })
                .ok_or_else(|| format!("judgement aura {label} is not registered"))
        })
        .collect::<Result<_, _>>()?;
    Ok(JudgementRefresh {
        judgements,
        white_swings_only: false,
    })
}

impl JudgementRefresh {
    /// The judgement auras, in Go's order.
    pub(crate) fn judgements(&self) -> &[AuraRef] {
        &self.judgements
    }

    pub(crate) fn on_spell_hit_dealt<A: Agent>(
        &self,
        fight: &mut Fight<A>,
        spell: SpellId,
        result: &SpellResult,
    ) {
        let state = &fight.spells[spell];
        if state.flags.proc || !state.melee_proc || result.outcome & OUTCOME_LANDED == 0 {
            return;
        }
        if self.white_swings_only
            && !state.white_hit
            && !fight
                .windfury_strikes
                .iter()
                .any(|&(strike, _)| strike == spell)
        {
            return;
        }
        for &aura in &self.judgements {
            let aura = fight.aura_on(aura, result.target);
            if fight.aura(aura).active {
                fight.refresh_aura(aura);
            }
        }
    }
}
