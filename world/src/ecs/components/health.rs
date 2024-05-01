use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use shipyard::Component;

use crate::entities::{internal_values::InternalValues, update_fields::UnitFields};

// Note: "Powers" includes health
#[derive(Component)]
pub struct Powers {
    internal_values: Arc<RwLock<InternalValues>>,
    next_regen_time: Instant,
}

impl Powers {
    pub fn new(
        current_health: u32,
        max_health: u32,
        internal_values: Arc<RwLock<InternalValues>>,
    ) -> Self {
        {
            let mut guard = internal_values.write();
            guard.set_u32(UnitFields::UnitFieldHealth.into(), current_health);
            guard.set_u32(UnitFields::UnitFieldMaxHealth.into(), max_health);
            guard.set_u32(UnitFields::UnitFieldBaseHealth.into(), max_health);
        }

        Self {
            internal_values,
            next_regen_time: Instant::now(),
        }
    }

    pub fn max_health(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldMaxHealth.into())
    }

    pub fn current_health(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldHealth.into())
    }

    pub fn apply_damage(&mut self, damage: u32) {
        let new_health = self.current_health().saturating_sub(damage);
        self.internal_values
            .write()
            .set_u32(UnitFields::UnitFieldHealth.into(), new_health);
    }

    pub fn apply_healing(&mut self, healing: u32) {
        let new_health = self
            .current_health()
            .saturating_add(healing)
            .min(self.max_health());
        self.internal_values
            .write()
            .set_u32(UnitFields::UnitFieldHealth.into(), new_health);
    }

    pub fn is_alive(&self) -> bool {
        self.current_health() > 0
    }

    pub fn reset_next_regen_time(&mut self) {
        self.next_regen_time = Instant::now() + Duration::from_secs(2);
    }

    pub fn is_past_next_regen_time(&self) -> bool {
        self.next_regen_time <= Instant::now()
    }
}
