use std::sync::Arc;

use parking_lot::RwLock;
use shipyard::Component;

use crate::entities::{internal_values::InternalValues, update_fields::UnitFields};

#[derive(Component)]
pub struct Health {
    internal_values: Arc<RwLock<InternalValues>>,
}

impl Health {
    pub fn new(current: u32, max: u32, internal_values: Arc<RwLock<InternalValues>>) -> Self {
        {
            let mut guard = internal_values.write();
            guard.set_u32(UnitFields::UnitFieldHealth.into(), current);
            guard.set_u32(UnitFields::UnitFieldMaxHealth.into(), max);
            guard.set_u32(UnitFields::UnitFieldBaseHealth.into(), max);
        }

        Self { internal_values }
    }

    pub fn max(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldMaxHealth.into())
    }

    pub fn current(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldHealth.into())
    }

    pub fn apply_damage(&mut self, damage: u32) {
        let new_health = self.current().saturating_sub(damage);
        self.internal_values
            .write()
            .set_u32(UnitFields::UnitFieldHealth.into(), new_health);

        // TODO: Handle threat based on the damage
        // See Unit.cpp:1014
    }

    pub fn apply_healing(&mut self, healing: u32) {
        let new_health = self.current().saturating_add(healing).min(self.max());
        self.internal_values
            .write()
            .set_u32(UnitFields::UnitFieldHealth.into(), new_health);
    }

    pub fn is_alive(&self) -> bool {
        self.current() > 0
    }
}
