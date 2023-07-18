use std::sync::Arc;

use log::warn;
use parking_lot::RwLock;
use shipyard::{Component, EntityId};

use crate::{
    entities::{internal_values::InternalValues, update_fields::UnitFields},
    shared::constants::UnitStandState,
};

#[derive(Component)]
pub struct Unit {
    target: Option<EntityId>,
    internal_values: Arc<RwLock<InternalValues>>,
    stand_state: UnitStandState,
}

impl Unit {
    pub fn new(internal_values: Arc<RwLock<InternalValues>>) -> Self {
        internal_values.write().set_u8(
            UnitFields::UnitFieldBytes1.into(),
            0,
            UnitStandState::Stand as u8,
        );

        Self {
            target: None,
            internal_values,
            stand_state: UnitStandState::Stand,
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
}
