use std::sync::Arc;

use shipyard::{AllStoragesViewMut, ViewMut};

use crate::{
    datastore::data_types::SpellRecord,
    ecs::components::{health::Health, threat_list::ThreatList},
};

use super::{
    map::Map, spell::Spell, spell_effect_handler::SpellEffectHandler, world_context::WorldContext,
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
        _world_context: Arc<WorldContext>,
        spell: Arc<Spell>,
        _map: Arc<Map>,
        spell_record: Arc<SpellRecord>,
        effect_index: usize,
        all_storages: &AllStoragesViewMut,
    ) {
        all_storages.run(
            |mut vm_health: ViewMut<Health>, mut vm_threat_list: ViewMut<ThreatList>| {
                let damage = spell_record.calc_simple_value(effect_index);
                vm_health[spell.target()].apply_damage(damage as u32);
                // TODO: use vm_threat_list.get for the case when it's a creature casting a spell
                // on a player (see how it's done for melee attacks)
                vm_threat_list[spell.target()].modify_threat(spell.caster(), damage as f32);
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
