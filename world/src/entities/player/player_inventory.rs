use std::collections::HashMap;

use log::warn;

use crate::{
    datastore::data_types::ItemTemplate,
    entities::{
        item::Item,
        update::{CreateData, UpdateData},
    },
    shared::constants::{InventorySlot, InventoryType},
};

// FIXME: Store &Item instead
pub struct PlayerInventory {
    items: HashMap<u32, Item>, // Key is slot
}

impl PlayerInventory {
    pub fn new(items: HashMap<u32, Item>) -> Self {
        Self { items }
    }

    pub fn build_create_data(&self) -> Vec<CreateData> {
        self.items
            .iter()
            .map(|item| item.1.build_create_data())
            .collect()
    }

    pub fn get(&self, slot: u32) -> Option<&Item> {
        self.items.get(&slot)
    }

    pub fn get_mut(&mut self, slot: u32) -> Option<&mut Item> {
        self.items.get_mut(&slot)
    }

    pub fn set(&mut self, slot: u32, item: Item) {
        self.items.insert(slot, item);
    }

    pub fn remove(&mut self, slot: u32) -> Option<Item> {
        self.items.remove(&slot)
    }

    pub fn list(&self) -> &HashMap<u32, Item> {
        &self.items
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

    pub fn swap(&mut self, slot1: u32, slot2: u32) {
        let item1 = self.items.get_mut(&slot1).unwrap() as *mut Item;
        let item2 = self.items.get_mut(&slot2).unwrap() as *mut Item;

        unsafe {
            std::ptr::swap(item1, item2);
        }
    }

    pub fn move_item(&mut self, from_slot: u32, to_slot: u32) {
        if let Some(item) = self.items.remove(&from_slot) {
            self.items.insert(to_slot, item);
        } else {
            warn!("attempt to move an item in inventory but item is not there");
        }
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
}
