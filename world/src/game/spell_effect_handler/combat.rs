use log::warn;
use shipyard::{Get, View, ViewMut};

use crate::{
    ecs::components::{guid::Guid, powers::Powers, threat_list::ThreatList, unit::Unit},
    entities::{creature::Creature, player::Player},
    game::{
        experience::Experience,
        spell_effect_handler::{SpellEffectHandler, SpellEffectHandlerArgs},
    },
    shared::constants::UnitDynamicFlag,
};

impl SpellEffectHandler {
    pub fn handle_effect_school_damage(
        SpellEffectHandlerArgs {
            spell,
            map_record,
            spell_record,
            effect_index,
            all_storages,
            ..
        }: SpellEffectHandlerArgs,
    ) {
        all_storages.run(
            |mut vm_powers: ViewMut<Powers>,
             mut vm_threat_list: ViewMut<ThreatList>,
             v_guid: View<Guid>,
             mut vm_player: ViewMut<Player>,
             v_creature: View<Creature>,
             v_unit: View<Unit>| {
                let Some(unit_target) = spell.unit_target() else {
                    warn!("handle_effect_school_damage: no unit target");
                    return;
                };

                let damage = spell_record.calc_simple_value(effect_index);
                let target_powers = &mut vm_powers[unit_target];
                target_powers.apply_damage(damage as u32);
                // TODO: Log damage somehow

                if target_powers.is_alive() {
                    if let Ok(mut threat_list) = (&mut vm_threat_list).get(unit_target) {
                        threat_list.modify_threat(spell.caster(), damage as f32);
                    }
                } else if let Ok(mut player) = (&mut vm_player).get(spell.caster()) {
                    // FIXME: This logic is duplicated in melee.rs
                    let target_guid = v_guid[unit_target].0;
                    let mut has_loot = false; // TODO: Handle player case (Insignia looting in PvP)
                    if let Ok(creature) = v_creature.get(unit_target) {
                        let xp_gain = Experience::xp_gain_against(&player, creature, map_record);
                        player.give_experience(xp_gain, Some(target_guid));
                        player.notify_killed_creature(creature.guid(), creature.template.entry);

                        has_loot = creature.generate_loot();
                    }

                    if let Ok(target_unit) = v_unit.get(unit_target) {
                        if has_loot {
                            target_unit.set_dynamic_flag(UnitDynamicFlag::Lootable);
                        }
                    }

                    player.unset_in_combat_with(target_guid);
                }
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
