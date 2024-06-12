use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use shipyard::Component;

use crate::{
    entities::{internal_values::InternalValues, update_fields::UnitFields},
    shared::constants::PowerType,
};

// Note: "Powers" includes health
#[derive(Component)]
pub struct Powers {
    internal_values: Arc<InternalValues>,
    next_regen_time: Instant,
    base_mana: u32,
}

pub struct PowerSnapshot {
    pub current: u32,
    pub max: u32,
}

pub struct PowersSnapshot {
    pub mana: PowerSnapshot,
    pub rage: PowerSnapshot,
    pub focus: PowerSnapshot,
    pub energy: PowerSnapshot,
    pub happiness: PowerSnapshot,
}

impl Powers {
    // Calculate all base/max power types
    pub fn new(internal_values: Arc<InternalValues>, base_health: u32, base_mana: u32) -> Self {
        internal_values.set_u32(UnitFields::UnitFieldBaseHealth.into(), base_health);
        internal_values.set_u32(UnitFields::UnitFieldBaseMana.into(), base_mana);

        Self {
            internal_values,
            next_regen_time: Instant::now(),
            base_mana,
        }
    }

    pub fn max_health(&self) -> u32 {
        self.internal_values
            .get_u32(UnitFields::UnitFieldMaxHealth.into())
    }

    pub fn base_health(&self) -> u32 {
        self.internal_values
            .get_u32(UnitFields::UnitFieldBaseHealth.into())
    }

    pub fn current_health(&self) -> u32 {
        self.internal_values
            .get_u32(UnitFields::UnitFieldHealth.into())
    }

    pub fn heal_to_max(&self) {
        let max_health = self.max_health();
        self.internal_values
            .set_u32(UnitFields::UnitFieldHealth.into(), max_health);
    }

    pub fn apply_damage(&mut self, damage: u32) {
        let new_health = self.current_health().saturating_sub(damage);
        self.internal_values
            .set_u32(UnitFields::UnitFieldHealth.into(), new_health);
    }

    pub fn apply_healing(&mut self, healing: u32) {
        let new_health = self
            .current_health()
            .saturating_add(healing)
            .min(self.max_health());
        self.internal_values
            .set_u32(UnitFields::UnitFieldHealth.into(), new_health);
    }

    pub fn modify_power(&self, power_type: &PowerType, diff: i32) {
        let index_current = UnitFields::UnitFieldPower1 as usize + *power_type as usize;
        let current_power = self.internal_values.get_u32(index_current);
        let max_power = self
            .internal_values
            .get_u32(UnitFields::UnitFieldMaxPower1 as usize + *power_type as usize);
        let new_value = current_power.saturating_add_signed(diff).min(max_power);
        self.internal_values.set_u32(index_current, new_value);
    }

    pub fn current_power(&self, power_type: &PowerType) -> u32 {
        self.internal_values
            .get_u32(UnitFields::UnitFieldPower1 as usize + *power_type as usize)
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

    pub fn base_mana(&self) -> u32 {
        self.base_mana
    }

    pub fn snapshot(&self) -> PowersSnapshot {
        let values = &self.internal_values;

        fn get_current_power(power_type: &PowerType, values: &InternalValues) -> u32 {
            values.get_u32(UnitFields::UnitFieldPower1 as usize + *power_type as usize)
        }

        fn get_max_power(power_type: &PowerType, values: &InternalValues) -> u32 {
            values.get_u32(UnitFields::UnitFieldMaxPower1 as usize + *power_type as usize)
        }

        PowersSnapshot {
            mana: PowerSnapshot {
                current: get_current_power(&PowerType::Mana, values),
                max: get_max_power(&PowerType::Mana, values),
            },
            rage: PowerSnapshot {
                current: get_current_power(&PowerType::Rage, values),
                max: get_max_power(&PowerType::Rage, values),
            },
            focus: PowerSnapshot {
                current: get_current_power(&PowerType::Focus, values),
                max: get_max_power(&PowerType::Focus, values),
            },
            energy: PowerSnapshot {
                current: get_current_power(&PowerType::Energy, values),
                max: get_max_power(&PowerType::Energy, values),
            },
            happiness: PowerSnapshot {
                current: get_current_power(&PowerType::PetHappiness, values),
                max: get_max_power(&PowerType::PetHappiness, values),
            },
        }
    }
}
