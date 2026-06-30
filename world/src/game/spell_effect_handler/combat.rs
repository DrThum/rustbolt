use log::warn;
use shipyard::{UniqueViewMut, ViewMut};

use crate::{
    ecs::{
        components::{powers::Powers, threat_list::ThreatList},
        resources::CombatEvents,
        systems::combat::apply_combat_damage,
    },
    game::spell_effect_handler::{SpellEffectHandler, SpellEffectHandlerArgs},
};

impl SpellEffectHandler {
    pub fn handle_effect_school_damage(
        SpellEffectHandlerArgs {
            spell,
            spell_record,
            effect_index,
            all_storages,
            ..
        }: SpellEffectHandlerArgs,
    ) {
        all_storages.run(
            |mut vm_powers: ViewMut<Powers>,
             mut vm_threat_list: ViewMut<ThreatList>,
             mut combat_events: UniqueViewMut<CombatEvents>| {
                let Some(unit_target) = spell.unit_target() else {
                    warn!("handle_effect_school_damage: no unit target");
                    return;
                };

                let damage = spell_record.calc_simple_value(effect_index) as f32;
                apply_combat_damage(
                    spell.caster(),
                    unit_target,
                    damage,
                    &mut vm_powers,
                    &mut vm_threat_list,
                    &mut combat_events,
                );
            },
        );
    }

    pub fn handle_effect_heal(
        SpellEffectHandlerArgs {
            spell,
            spell_record,
            effect_index,
            all_storages,
            ..
        }: SpellEffectHandlerArgs,
    ) {
        all_storages.run(|mut vm_powers: ViewMut<Powers>| {
            let Some(unit_target) = spell.unit_target() else {
                warn!("handle_effect_school_damage: no unit target");
                return;
            };

            let damage = spell_record.calc_simple_value(effect_index);
            vm_powers[unit_target].apply_healing(damage as u32);
        });
    }
}
