use std::sync::Arc;

use shipyard::{AllStoragesViewMut, Get, View, ViewMut};

use crate::{
    datastore::data_types::SpellRecord,
    ecs::components::{guid::Guid, health::Health, threat_list::ThreatList, unit::Unit},
    entities::{creature::Creature, player::Player},
    shared::constants::UnitDynamicFlag,
};

use super::{
    experience::Experience, map::Map, spell::Spell, spell_effect_handler::SpellEffectHandler,
    world_context::WorldContext,
};

impl SpellEffectHandler {
    pub(crate) fn unhandled(
        _world_context: Arc<WorldContext>,
        _spell: Arc<Spell>,
        _map: Arc<Map>,
        _spell_record: Arc<SpellRecord>,
        _effect_index: usize,
        _all_storages: &AllStoragesViewMut,
    ) {
    }

    pub fn handle_effect_school_damage(
        world_context: Arc<WorldContext>,
        spell: Arc<Spell>,
        map: Arc<Map>,
        spell_record: Arc<SpellRecord>,
        effect_index: usize,
        all_storages: &AllStoragesViewMut,
    ) {
        all_storages.run(
            |mut vm_health: ViewMut<Health>,
             mut vm_threat_list: ViewMut<ThreatList>,
             v_guid: View<Guid>,
             mut vm_player: ViewMut<Player>,
             v_creature: View<Creature>,
             mut vm_unit: ViewMut<Unit>| {
                let damage = spell_record.calc_simple_value(effect_index);
                let target_health = &mut vm_health[spell.target()];
                target_health.apply_damage(damage as u32);
                // TODO: Log damage somehow

                if target_health.is_alive() {
                    if let Ok(mut threat_list) = (&mut vm_threat_list).get(spell.target()) {
                        threat_list.modify_threat(spell.caster(), damage as f32);
                    }
                } else if let Ok(mut player) = (&mut vm_player).get(spell.caster()) {
                    let target_guid = v_guid[spell.target()].0;
                    if let Ok(creature) = v_creature.get(spell.target()) {
                        let xp_gain = Experience::xp_gain_against(
                            &player,
                            creature,
                            map.id(),
                            world_context.data_store.clone(),
                        );
                        player.give_experience(xp_gain, Some(target_guid));
                        player.notify_killed_creature(creature.guid(), creature.template.entry);
                    }

                    if let Ok(target_unit) = (&mut vm_unit).get(spell.target()) {
                        target_unit.set_dynamic_flag(UnitDynamicFlag::Lootable);
                    }

                    player.unset_in_combat_with(target_guid);
                }
            },
        );
    }

    pub fn handle_effect_heal(
        _world_context: Arc<WorldContext>,
        _spell: Arc<Spell>,
        _map: Arc<Map>,
        _spell_record: Arc<SpellRecord>,
        effect_index: usize,
        all_storages: &AllStoragesViewMut,
    ) {
        all_storages.run(|mut vm_health: ViewMut<Health>| {
            let damage = _spell_record.calc_simple_value(effect_index);
            vm_health[_spell.target()].apply_healing(damage as u32);
        });
    }
}
