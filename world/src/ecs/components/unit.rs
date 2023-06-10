use std::sync::Arc;

use parking_lot::RwLock;
use shipyard::{Component, EntityId};

use crate::entities::{internal_values::InternalValues, update_fields::UnitFields};

#[derive(Component)]
pub struct Unit {
    target: Option<EntityId>,
    internal_values: Arc<RwLock<InternalValues>>,
}

impl Unit {
    pub fn new(internal_values: Arc<RwLock<InternalValues>>) -> Self {
        Self {
            target: None,
            internal_values,
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
}
