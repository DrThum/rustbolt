use std::{collections::HashSet, sync::Arc};

use parking_lot::RwLock;
use shipyard::Component;

use crate::shared::constants::{
    AttributeModifier, AttributeModifierType, PowerType, SpellSchool, UnitAttribute,
};

use super::{internal_values::InternalValues, update_fields::UnitFields};

#[derive(Component)]
pub struct Attributes {
    internal_values: Arc<RwLock<InternalValues>>,
    modifiers: [[f32; AttributeModifierType::Max as usize]; AttributeModifier::Max as usize],
    dirty: HashSet<AttributeModifier>,
}

impl Attributes {
    pub fn new(internal_values: Arc<RwLock<InternalValues>>) -> Self {
        let mut start_values = [[0., 1., 0., 1.]; AttributeModifier::Max as usize];
        // Off hand deals 50% base damage
        start_values[AttributeModifier::DamageOffHand as usize]
            [AttributeModifierType::TotalPercent as usize] = 0.5;

        Self {
            internal_values,
            modifiers: start_values,
            dirty: HashSet::new(),
        }
    }

    pub fn dirty_modifiers(&self) -> HashSet<AttributeModifier> {
        self.dirty.clone()
    }

    pub fn reset_dirty(&mut self) {
        self.dirty.clear();
    }

    pub fn add_modifier(
        &mut self,
        modifier: AttributeModifier,
        modifier_type: AttributeModifierType,
        value: f32,
    ) {
        self.modifiers[modifier as usize][modifier_type as usize] += value;
        self.dirty.insert(modifier);

        // Some modifiers trigger changes to another modifiers
        let extra_modifiers: Vec<AttributeModifier> = match modifier {
            AttributeModifier::StatAgility => vec![AttributeModifier::Armor],
            AttributeModifier::StatStamina => vec![AttributeModifier::Health],
            AttributeModifier::StatIntellect => vec![AttributeModifier::Mana],
            _ => vec![],
        };

        for extra_modifier in extra_modifiers {
            self.dirty.insert(extra_modifier);
        }
    }

    pub fn total_modifier_value(&self, modifier: AttributeModifier) -> f32 {
        let relevant_modifier = self.modifiers[modifier as usize];
        let base = relevant_modifier[AttributeModifierType::BaseValue as usize];
        let base_percent = relevant_modifier[AttributeModifierType::BasePercent as usize];
        let total = relevant_modifier[AttributeModifierType::TotalValue as usize];
        let total_percent = relevant_modifier[AttributeModifierType::TotalPercent as usize];

        (((base * base_percent) + total) * total_percent).max(0.)
    }

    pub fn modifier_values(
        &self,
        modifier: AttributeModifier,
    ) -> &[f32; AttributeModifierType::Max as usize] {
        &self.modifiers[modifier as usize]
    }

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

    pub fn level(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldLevel.into())
    }
}
