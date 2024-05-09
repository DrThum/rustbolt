use std::collections::HashSet;

use crate::shared::constants::{AttributeModifier, AttributeModifierType};

pub struct AttributeModifiers {
    modifiers: [[f32; AttributeModifierType::Max as usize]; AttributeModifier::Max as usize],
    dirty: HashSet<AttributeModifier>,
}

impl AttributeModifiers {
    pub fn new() -> Self {
        let mut start_values = [[0., 1., 0., 1.]; AttributeModifier::Max as usize];
        // Off hand deals 50% base damage
        start_values[AttributeModifier::DamageOffHand as usize]
            [AttributeModifierType::TotalPercent as usize] = 0.5;

        Self {
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
}
