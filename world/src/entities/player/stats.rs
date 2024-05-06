use crate::shared::constants::{SpellSchool, UnitAttribute};

use super::{Player, UnitFields};

impl Player {
    // NOTE: MaNGOS uses f32 for internal calculation but client expects u32
    pub fn attribute(&self, attr: UnitAttribute) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldStat0 as usize + attr as usize)
    }

    pub fn resistance(&self, spell_school: SpellSchool) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldResistances as usize + spell_school as usize)
    }

    pub fn armor(&self) -> u32 {
        self.resistance(SpellSchool::Normal)
    }
}
