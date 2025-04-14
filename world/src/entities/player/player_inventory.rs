use std::{collections::HashMap, sync::Arc};

use log::{error, warn};
use parking_lot::RwLock;

use crate::{
    datastore::data_types::ItemTemplate,
    entities::{
        attribute_modifiers::AttributeModifiers,
        internal_values::InternalValues,
        item::Item,
        object_guid::ObjectGuid,
        update::{CreateData, UpdateData},
    },
    shared::constants::{AttributeModifier, AttributeModifierType, InventorySlot, InventoryType},
    DataStore,
};

use super::{UnitFields, MAX_PLAYER_VISIBLE_ITEM_OFFSET};

pub struct PlayerInventory {
    items: HashMap<u32, Item>, // Key is slot
    internal_values: Arc<RwLock<InternalValues>>,
    attribute_modifiers: Arc<RwLock<AttributeModifiers>>,
    data_store: Arc<DataStore>,
}

impl PlayerInventory {
    pub fn new(
        internal_values: Arc<RwLock<InternalValues>>,
        attribute_modifiers: Arc<RwLock<AttributeModifiers>>,
        data_store: Arc<DataStore>,
    ) -> Self {
        Self {
            items: HashMap::new(),
            internal_values,
            attribute_modifiers,
            data_store,
        }
    }

    pub fn build_create_data(&self) -> Vec<CreateData> {
        self.items
            .iter()
            .map(|item| item.1.build_create_data())
            .collect()
    }

    pub fn list(&self) -> &HashMap<u32, Item> {
        &self.items
    }

    pub fn get(&self, slot: u32) -> Option<&Item> {
        self.items.get(&slot)
    }

    pub fn get_mut_by_guid(&mut self, guid: ObjectGuid) -> Option<(&u32, &mut Item)> {
        self.items.iter_mut().find(|(_, item)| *item.guid() == guid)
    }

    pub fn has_item_in_slot(&self, slot: u32) -> bool {
        self.items.contains_key(&slot)
    }

    pub fn get_mut(&mut self, slot: u32) -> Option<&mut Item> {
        self.items.get_mut(&slot)
    }

    pub fn get2_mut(&mut self, slot1: u32, slot2: u32) -> (Option<&mut Item>, Option<&mut Item>) {
        if slot1 == slot2 {
            error!("PlayerInventory::get2_mut called with slot1 == slot2");
            return (None, None);
        }

        let has_slot1 = self.items.contains_key(&slot1);
        let has_slot2 = self.items.contains_key(&slot2);

        unsafe {
            match (has_slot1, has_slot2) {
                (false, false) => (None, None),
                (true, false) => {
                    let item1 = self.items.get_mut(&slot1).unwrap() as *mut _;
                    (Some(&mut *item1), None)
                }
                (true, true) => {
                    let item1 = self.items.get_mut(&slot1).unwrap() as *mut _;
                    let item2 = self.items.get_mut(&slot2).unwrap() as *mut _;
                    (Some(&mut *item1), Some(&mut *item2))
                }
                (false, true) => {
                    let item2 = self.items.get_mut(&slot2).unwrap() as *mut _;
                    (None, Some(&mut *item2))
                }
            }
        }
    }

    pub fn set(&mut self, slot: u32, item: Item) {
        if self.items.contains_key(&slot) {
            error!("PlayerInventory: attempt to overwrite slot {slot}");
            return;
        }

        self.internal_values.write().set_guid(
            UnitFields::PlayerFieldInvSlotHead as usize + (2 * slot) as usize,
            item.guid(),
        );

        self.update_visible_bits(slot, item.entry());

        let item_entry = item.entry();
        self.items.insert(slot, item);

        if Self::is_gear_slot(slot) {
            self.toggle_stats_from_item(item_entry, true);
        }
    }

    pub fn remove(&mut self, slot: u32) -> Option<Item> {
        self.internal_values.write().set_u64(
            UnitFields::PlayerFieldInvSlotHead as usize + (2 * slot) as usize,
            0,
        );

        self.update_visible_bits(slot, 0);

        self.items.remove(&slot).map(|removed_item| {
            if Self::is_gear_slot(slot) {
                self.toggle_stats_from_item(removed_item.entry(), false);
            }

            removed_item
        })
    }

    pub fn remove_item_count(&mut self, item_id: u32, stack_count: u32) {
        let mut remaining_stacks_to_remove = stack_count;
        for slot in InventorySlot::BACKPACK_START..InventorySlot::BACKPACK_END {
            if remaining_stacks_to_remove == 0 {
                break;
            }

            let Some(item) = self.items.get_mut(&slot) else {
                continue;
            };

            if item.entry() != item_id {
                continue;
            }

            // Unstack the item if it has more stacks than needed and stop there
            if item.stack_count() > remaining_stacks_to_remove {
                item.change_stack_count(-(remaining_stacks_to_remove as i32));
                break;
            }

            // Otherwise, remove the item and try to remove the rest of the stacks from somewhere else
            let removed_item = self.remove(slot);
            remaining_stacks_to_remove -= removed_item.map(|item| item.stack_count()).unwrap_or(0);
        }
    }

    pub fn swap(&mut self, source_slot: u32, destination_slot: u32) {
        let destination_item = self.remove(destination_slot);

        self.move_item(source_slot, destination_slot);
        if let Some(destination_item) = destination_item {
            self.set(source_slot, destination_item);
        }
    }

    pub fn move_item(&mut self, source_slot: u32, destination_slot: u32) {
        if let Some(item) = self.items.remove(&source_slot) {
            {
                let mut values = self.internal_values.write();
                values.set_u64(
                    UnitFields::PlayerFieldInvSlotHead as usize + (2 * source_slot) as usize,
                    0,
                );

                values.set_guid(
                    UnitFields::PlayerFieldInvSlotHead as usize + (2 * destination_slot) as usize,
                    item.guid(),
                );
            }

            let item_entry = item.entry();
            self.update_visible_bits(source_slot, 0);
            self.update_visible_bits(destination_slot, item_entry);

            self.items.insert(destination_slot, item);

            let is_moved_from_gear = Self::is_gear_slot(source_slot);
            let is_moved_to_gear = Self::is_gear_slot(destination_slot);

            // Remove stats from the item if source is gear and destination is not (item unequipped)
            // Add stats from the item if destination is gear and source is not (item equipped)
            if is_moved_from_gear && !is_moved_to_gear {
                self.toggle_stats_from_item(item_entry, false);
            } else if is_moved_to_gear && !is_moved_from_gear {
                self.toggle_stats_from_item(item_entry, true);
            }
        } else {
            warn!("attempt to move an item in inventory but item is not there");
        }
    }

    pub fn mark_saved(&mut self) {
        for item in self.items.values_mut() {
            item.mark_saved();
        }
    }

    pub fn list_updated_and_reset(&mut self) -> Vec<UpdateData> {
        let mut all_update_data = Vec::new();
        for (_, item) in self.items.iter_mut() {
            if let Some(item_update_data) = item.build_update_data_and_reset() {
                all_update_data.push(item_update_data);
            }
        }
        all_update_data
    }

    pub fn equipment_slot_for(&self, item_template: &ItemTemplate) -> Option<InventorySlot> {
        // FIXME: It's more complex than that
        fn calculate_weapon_slot() -> Option<InventorySlot> {
            Some(InventorySlot::EquipmentMainHand)
        }

        if let Some(inventory_type) = InventoryType::n(item_template.inventory_type) {
            return match inventory_type {
                InventoryType::NonEquip => None,
                InventoryType::Head => Some(InventorySlot::EquipmentHead),
                InventoryType::Neck => Some(InventorySlot::EquipmentNeck),
                InventoryType::Shoulders => Some(InventorySlot::EquipmentShoulders),
                InventoryType::Body => Some(InventorySlot::EquipmentBody),
                InventoryType::Chest => Some(InventorySlot::EquipmentChest),
                InventoryType::Waist => Some(InventorySlot::EquipmentWaist),
                InventoryType::Legs => Some(InventorySlot::EquipmentLegs),
                InventoryType::Feet => Some(InventorySlot::EquipmentFeet),
                InventoryType::Wrists => Some(InventorySlot::EquipmentWrists),
                InventoryType::Hands => Some(InventorySlot::EquipmentHands),
                InventoryType::Finger => {
                    let player_has_finger1 = self
                        .items
                        .contains_key(&(InventorySlot::EquipmentFinger1 as u32));
                    let player_has_finger2 = self
                        .items
                        .contains_key(&(InventorySlot::EquipmentFinger2 as u32));

                    if player_has_finger1 && !player_has_finger2 {
                        return Some(InventorySlot::EquipmentFinger2);
                    }

                    return Some(InventorySlot::EquipmentFinger1);
                }
                InventoryType::Trinket => {
                    let player_has_trinket1 = self
                        .items
                        .contains_key(&(InventorySlot::EquipmentTrinket1 as u32));
                    let player_has_trinket2 = self
                        .items
                        .contains_key(&(InventorySlot::EquipmentTrinket2 as u32));

                    if player_has_trinket1 && !player_has_trinket2 {
                        return Some(InventorySlot::EquipmentTrinket2);
                    }

                    return Some(InventorySlot::EquipmentTrinket1);
                }
                InventoryType::Weapon => calculate_weapon_slot(),
                InventoryType::Shield => Some(InventorySlot::EquipmentOffHand),
                InventoryType::Ranged => Some(InventorySlot::EquipmentRanged),
                InventoryType::Cloak => Some(InventorySlot::EquipmentBack),
                InventoryType::TwoHandWeapon => calculate_weapon_slot(),
                InventoryType::Bag => todo!(),
                InventoryType::Tabard => Some(InventorySlot::EquipmentTabard),
                InventoryType::Robe => Some(InventorySlot::EquipmentChest),
                InventoryType::WeaponMainHand => Some(InventorySlot::EquipmentMainHand),
                InventoryType::WeaponOffHand => Some(InventorySlot::EquipmentOffHand),
                InventoryType::Holdable => Some(InventorySlot::EquipmentOffHand),
                InventoryType::Ammo => todo!(),
                InventoryType::Thrown => Some(InventorySlot::EquipmentRanged),
                InventoryType::RangedRight => Some(InventorySlot::EquipmentRanged),
                InventoryType::Quiver => todo!(),
                InventoryType::Relic => todo!(),
            };
        }

        None
    }

    pub fn find_first_free_slot(&self) -> Option<u32> {
        (InventorySlot::BACKPACK_START..InventorySlot::BACKPACK_END)
            .find(|&slot| !self.items.contains_key(&slot))
    }

    fn update_visible_bits(&self, slot: u32, item_entry: u32) {
        let mut values = self.internal_values.write();

        if (InventorySlot::EQUIPMENT_START..InventorySlot::EQUIPMENT_END).contains(&slot) {
            values.set_u32(
                UnitFields::PlayerVisibleItem1_0 as usize
                    + (slot * MAX_PLAYER_VISIBLE_ITEM_OFFSET) as usize,
                item_entry,
            );
        }
    }

    pub fn get_item_count(&self, item_id: u32) -> u32 {
        self.items
            .values()
            .filter_map(|item| {
                if item.entry() == item_id {
                    Some(item.stack_count())
                } else {
                    None
                }
            })
            .sum()
    }

    fn is_gear_slot(slot: u32) -> bool {
        (InventorySlot::EQUIPMENT_START..InventorySlot::EQUIPMENT_END).contains(&slot)
    }

    fn toggle_stats_from_item(&self, item_entry: u32, is_equipping_item: bool) {
        // If we are unequipping the item, we want to _remove_ the stats
        let factor = if is_equipping_item { 1.0 } else { -1.0 };

        if let Some(item_template) = self.data_store.get_item_template(item_entry) {
            let mut attr_mod = self.attribute_modifiers.write();
            // Stats: both generic (stam, spirit, intel, ...) and green modifiers
            for stat in &item_template.stats {
                if let Some(attribute_modifier) = stat.stat_type.as_attribute_modifier() {
                    attr_mod.add_modifier(
                        attribute_modifier,
                        AttributeModifierType::BaseValue,
                        stat.stat_value as f32 * factor,
                    );
                }
            }

            // Armor
            if item_template.armor != 0 {
                attr_mod.add_modifier(
                    AttributeModifier::Armor,
                    AttributeModifierType::BaseValue,
                    item_template.armor as f32 * factor,
                );
            }

            // Resistances
            if item_template.holy_res != 0 {
                attr_mod.add_modifier(
                    AttributeModifier::ResistanceHoly,
                    AttributeModifierType::BaseValue,
                    item_template.holy_res as f32 * factor,
                );
            }

            if item_template.fire_res != 0 {
                attr_mod.add_modifier(
                    AttributeModifier::ResistanceFire,
                    AttributeModifierType::BaseValue,
                    item_template.fire_res as f32 * factor,
                );
            }

            if item_template.nature_res != 0 {
                attr_mod.add_modifier(
                    AttributeModifier::ResistanceNature,
                    AttributeModifierType::BaseValue,
                    item_template.nature_res as f32 * factor,
                );
            }

            if item_template.frost_res != 0 {
                attr_mod.add_modifier(
                    AttributeModifier::ResistanceFrost,
                    AttributeModifierType::BaseValue,
                    item_template.frost_res as f32 * factor,
                );
            }

            if item_template.shadow_res != 0 {
                attr_mod.add_modifier(
                    AttributeModifier::ResistanceShadow,
                    AttributeModifierType::BaseValue,
                    item_template.shadow_res as f32 * factor,
                );
            }

            if item_template.arcane_res != 0 {
                attr_mod.add_modifier(
                    AttributeModifier::ResistanceArcane,
                    AttributeModifierType::BaseValue,
                    item_template.arcane_res as f32 * factor,
                );
            }
        }
    }
}
