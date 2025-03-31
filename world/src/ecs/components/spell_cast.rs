use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use log::{error, warn};
use shipyard::{Component, EntityId, View, ViewMut};

use crate::{
    entities::object_guid::ObjectGuid,
    game::{
        map::Map, spell::Spell, spell_cast_target::SpellCastTargets, world_context::WorldContext,
    },
    shared::constants::SpellFailReason,
};

use super::powers::Powers;

#[derive(Component)]
pub struct SpellCast {
    current_ranged: Option<Arc<Spell>>,
    ranged_cast_end: Option<Instant>,
    // current melee: Option<u32>,
}

impl SpellCast {
    pub fn new() -> Self {
        Self {
            current_ranged: None,
            ranged_cast_end: None,
        }
    }

    pub fn current_ranged(&self) -> Option<(Arc<Spell>, Instant)> {
        match (self.current_ranged.as_ref(), self.ranged_cast_end) {
            (None, None) => None,
            (Some(curr), Some(end)) => Some((curr.clone(), end)),
            _ => {
                error!("inconsistent state: current_ranged.is_some() != ranged_cast_end.is_some()");
                None
            }
        }
    }

    pub fn set_current_ranged(
        &mut self,
        spell_id: u32,
        duration: Duration,
        caster_entity_id: EntityId,
        caster_guid: ObjectGuid,
        unit_target: Option<EntityId>,
        game_object_target: Option<EntityId>,
        power_cost: u32,
    ) {
        self.current_ranged = Some(Arc::new(Spell::new(
            spell_id,
            caster_entity_id,
            caster_guid,
            unit_target,
            game_object_target,
            power_cost,
        )));
        self.ranged_cast_end = Some(Instant::now() + duration)
    }

    pub fn clean(&mut self) {
        self.current_ranged = None;
        self.ranged_cast_end = None;
    }

    pub fn cast_spell(
        map: Arc<Map>,
        world_context: Arc<WorldContext>,
        caster_guid: &ObjectGuid,
        spell_id: u32,
        targets: &mut SpellCastTargets,
    ) -> Result<SpellCastSuccess, SpellFailReason> {
        targets.update_internal_refs(map.clone());

        let Some(spell_record) = world_context.data_store.get_spell_record(spell_id) else {
            error!("SpellCast::cast_spell: unknown spell {}", spell_id);
            return Err(SpellFailReason::SpellUnavailable);
        };

        let spell_base_cast_time = spell_record
            .base_cast_time(world_context.data_store.clone())
            .unwrap();

        let Some(caster_entity_id) = map.lookup_entity_ecs(caster_guid) else {
            error!("SpellCast::cast_spell: no EntityId found for caster guid {caster_guid:?}");
            return Err(SpellFailReason::DontReport);
        };

        map.world()
            .run(|mut vm_spell: ViewMut<SpellCast>, v_powers: View<Powers>| {
                if vm_spell[caster_entity_id].current_ranged().is_some() {
                    return Err(SpellFailReason::SpellInProgress);
                }

                let Some(spell_record) = world_context.data_store.get_spell_record(spell_id) else {
                    warn!("attempt to cast non-existing spell {}", spell_id);
                    return Err(SpellFailReason::DontReport);
                };

                let powers = &v_powers[caster_entity_id];
                let power_cost = spell_record.calculate_power_cost(
                    powers.base_health(),
                    powers.base_mana(),
                    powers.snapshot(),
                );
                vm_spell[caster_entity_id].set_current_ranged(
                    spell_id,
                    spell_base_cast_time,
                    caster_entity_id,
                    *caster_guid,
                    targets.unit_target(),
                    targets.game_object_target(),
                    power_cost,
                );

                Ok(SpellCastSuccess {
                    spell_base_cast_time,
                })
            })
    }
}

pub struct SpellCastSuccess {
    pub spell_base_cast_time: Duration,
}
