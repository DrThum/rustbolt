use crate::shared::constants::{PowerType, SpellSchool, UnitAttribute};

use super::{Player, UnitFields};

impl Player {
    pub fn set_health_to_max(&self) {
        let max_health = self
            .internal_values
            .read()
            .get_u32(UnitFields::UnitFieldMaxHealth.into());
        self.internal_values
            .write()
            .set_u32(UnitFields::UnitFieldHealth.into(), max_health);
    }

    pub fn set_max_health(&self, value: u32) {
        self.internal_values
            .write()
            .set_u32(UnitFields::UnitFieldMaxHealth.into(), value);
    }

    pub fn set_mana_to_max(&self) {
        let max_mana = self
            .internal_values
            .read()
            .get_u32(UnitFields::UnitFieldMaxPower1.into());
        self.internal_values
            .write()
            .set_u32(UnitFields::UnitFieldPower1.into(), max_mana);
    }

    pub fn set_max_power(&self, power_type: PowerType, value: u32) {
        self.internal_values.write().set_u32(
            UnitFields::UnitFieldMaxPower1 as usize + power_type as usize,
            value,
        );
    }

    // NOTE: MaNGOS uses f32 for internal calculation but client expects u32
    pub fn attribute(&self, attr: UnitAttribute) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldStat0 as usize + attr as usize)
    }

    pub fn set_attribute(&self, attr: UnitAttribute, value: u32) {
        self.internal_values
            .write()
            .set_u32(UnitFields::UnitFieldStat0 as usize + attr as usize, value);
    }

    pub fn resistance(&self, spell_school: SpellSchool) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldResistances as usize + spell_school as usize)
    }

    pub fn set_resistance(&self, spell_school: SpellSchool, value: u32) {
        self.internal_values.write().set_u32(
            UnitFields::UnitFieldResistances as usize + spell_school as usize,
            value,
        );
    }
}
