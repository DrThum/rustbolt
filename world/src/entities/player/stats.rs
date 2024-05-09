use crate::shared::constants::{SpellSchool, UnitAttribute};

use super::{Player, UnitFields};

impl Player {
    // NOTE: MaNGOS uses f32 for internal calculation but client expects u32
    pub fn attribute(&self, attr: UnitAttribute) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldStat0 as usize + attr as usize)
    }

    pub fn set_attribute(&mut self, attr: UnitAttribute, value: u32) {
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

    pub fn armor(&self) -> u32 {
        self.resistance(SpellSchool::Normal)
    }

    pub fn recalculate_armor(&self) {
        println!("updating armor");
    }
}
