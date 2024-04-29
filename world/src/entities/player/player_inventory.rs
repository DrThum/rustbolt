use std::{collections::HashMap, sync::Arc};

use log::warn;
use parking_lot::RwLock;

use crate::{
    datastore::data_types::ItemTemplate,
    entities::{
        internal_values::InternalValues,
        item::Item,
        update::{CreateData, UpdateData},
    },
    shared::constants::{InventorySlot, InventoryType},
};

use super::{UnitFields, MAX_PLAYER_VISIBLE_ITEM_OFFSET};

// FIXME: Store &Item instead
pub struct PlayerInventory {
    items: HashMap<u32, Item>, // Key is slot
    internal_values: Arc<RwLock<InternalValues>>,
}

impl PlayerInventory {
    pub fn new(internal_values: Arc<RwLock<InternalValues>>) -> Self {
        Self {
            items: HashMap::new(),
            internal_values,
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

    pub fn has_item_in_slot(&self, slot: u32) -> bool {
        self.items.contains_key(&slot)
    }

    pub fn get_mut(&mut self, slot: u32) -> Option<&mut Item> {
        self.items.get_mut(&slot)
    }

    pub fn set(&mut self, slot: u32, item: Item) {
        self.internal_values.write().set_guid(
            UnitFields::PlayerFieldInvSlotHead as usize + (2 * slot) as usize,
            item.guid(),
        );

        self.update_visible_bits(slot, item.entry());

        self.items.insert(slot, item);
    }

    pub fn remove(&mut self, slot: u32) -> Option<Item> {
        self.internal_values.write().set_u64(
            UnitFields::PlayerFieldInvSlotHead as usize + (2 * slot) as usize,
            0,
        );

        self.update_visible_bits(slot, 0);

        self.items.remove(&slot)
    }

    pub fn swap(&mut self, source_slot: u32, destination_slot: u32) {
        let source_item = self.items.get_mut(&source_slot).unwrap() as *mut Item;
        let target_item = self.items.get_mut(&destination_slot).unwrap() as *mut Item;

        unsafe {
            {
                let mut values = self.internal_values.write();
                values.set_guid(
                    UnitFields::PlayerFieldInvSlotHead as usize + (2 * source_slot) as usize,
                    target_item.as_ref().unwrap().guid(),
                );

                values.set_guid(
                    UnitFields::PlayerFieldInvSlotHead as usize + (2 * destination_slot) as usize,
                    source_item.as_ref().unwrap().guid(),
                );
            }

            self.update_visible_bits(destination_slot, source_item.as_ref().unwrap().entry());
            self.update_visible_bits(source_slot, target_item.as_ref().unwrap().entry());

            std::ptr::swap(source_item, target_item);
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

            self.update_visible_bits(source_slot, 0);
            self.update_visible_bits(destination_slot, item.entry());

            self.items.insert(destination_slot, item);
        } else {
            warn!("attempt to move an item in inventory but item is not there");
        }
    }

    pub fn mark_saved(&mut self) {
        for (_, item) in &mut self.items {
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
        for slot in InventorySlot::BACKPACK_START..InventorySlot::BACKPACK_END {
            if !self.items.contains_key(&slot) {
                return Some(slot);
            }
        }

        None
    }

    fn update_visible_bits(&self, slot: u32, item_entry: u32) {
        let mut values = self.internal_values.write();

        if slot >= InventorySlot::EQUIPMENT_START && slot < InventorySlot::EQUIPMENT_END {
            values.set_u32(
                UnitFields::PlayerVisibleItem1_0 as usize
                    + (slot * MAX_PLAYER_VISIBLE_ITEM_OFFSET) as usize,
                item_entry,
            );
        }
    }
}
