use std::sync::Arc;

use log::warn;
use parking_lot::RwLock;
use shipyard::{Component, EntityId};

use crate::{
    entities::{internal_values::InternalValues, update_fields::UnitFields},
    shared::constants::{UnitFlags, UnitStandState},
    DataStore,
};

#[derive(Component)]
pub struct Unit {
    target: Option<EntityId>,
    internal_values: Arc<RwLock<InternalValues>>,
    stand_state: UnitStandState,
    data_store: Arc<DataStore>,
}

impl Unit {
    pub fn new(internal_values: Arc<RwLock<InternalValues>>, data_store: Arc<DataStore>) -> Self {
        internal_values.write().set_u8(
            UnitFields::UnitFieldBytes1.into(),
            0,
            UnitStandState::Stand as u8,
        );

        Self {
            target: None,
            internal_values,
            stand_state: UnitStandState::Stand,
            data_store,
        }
    }

    pub fn target(&self) -> Option<EntityId> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<EntityId>, raw_guid: u64) {
        self.target = target;
        self.internal_values
            .write()
            .set_u64(UnitFields::UnitFieldTarget.into(), raw_guid);
    }

    pub fn set_stand_state(&mut self, stand_state: u32) {
        if let Some(stand_state_enum) = UnitStandState::n(stand_state) {
            self.internal_values.write().set_u8(
                UnitFields::UnitFieldBytes1.into(),
                0,
                stand_state as u8,
            );
            self.stand_state = stand_state_enum;
        } else {
            warn!(
                "attempted to set an invalid stand state ({}) on unit",
                stand_state
            );
        }
    }

    pub fn set_combat_state(&self, set_in_combat: bool) {
        if set_in_combat {
            self.internal_values.write().set_flag_u32(
                UnitFields::UnitFieldFlags.into(),
                UnitFlags::InCombat as u32,
            );
        } else {
            self.internal_values.write().unset_flag_u32(
                UnitFields::UnitFieldFlags.into(),
                UnitFlags::InCombat as u32,
            );
        }
    }

    pub fn combat_state(&self) -> bool {
        self.internal_values.read().has_flag_u32(
            UnitFields::UnitFieldFlags.into(),
            UnitFlags::InCombat as u32,
        )
    }

    pub fn bounding_radius(&self) -> f32 {
        self.internal_values
            .read()
            .get_f32(UnitFields::UnitFieldBoundingRadius.into())
    }

    pub fn faction_id(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldFactionTemplate.into())
    }

    pub fn is_hostile_to(&self, other: &Unit) -> bool {
        if let Some(my_faction) = self
            .data_store
            .get_faction_template_record(self.faction_id())
        {
            if let Some(target_faction) = self
                .data_store
                .get_faction_template_record(other.faction_id())
            {
                return my_faction.is_hostile_to(target_faction);
            }
        }

        warn!("faction {} not found", self.faction_id());
        return false;
    }
}
