use log::error;

use crate::{
    entities::{item::Item, update::UpdateData},
    protocol::{
        packets::{SmsgCreateObject, SmsgItemPushResult},
        server::ServerMessage,
    },
    shared::constants::{InventoryResult, InventorySlot, PlayerQuestStatus},
};

use super::{player_inventory::PlayerInventory, Player};

impl Player {
    // Assume that we store in bags for now (TODO bank later)
    pub fn auto_store_new_item(
        &mut self,
        item_id: u32,
        stack_count: u32,
    ) -> Result<u32, InventoryResult> {
        let item_template = self
            .world_context
            .data_store
            .get_item_template(item_id)
            .expect("unknown item found in inventory");

        let mut remaining_stack_count = stack_count;

        // Attempt to distribute the new stacks over existing incomplete stacks
        // TODO: Implement bags, we only check in the backpack for now
        for slot in InventorySlot::BACKPACK_START..InventorySlot::BACKPACK_END {
            if let Some(item) = self.inventory.get_mut(slot) {
                if item.entry() == item_id && item.stack_count() < item_template.max_stack_count {
                    let available_stack_count = item_template.max_stack_count - item.stack_count();
                    let stack_count_to_add = remaining_stack_count.min(available_stack_count);
                    item.change_stack_count(stack_count_to_add.try_into().unwrap());
                    remaining_stack_count = remaining_stack_count - stack_count_to_add;
                }
            }

            if remaining_stack_count == 0 {
                break;
            }
        }

        // Drop the leftover in empty slots
        let mut chosen_slot = u32::MAX;
        while remaining_stack_count > 0 {
            match self.inventory.find_first_free_slot() {
                Some(slot) => {
                    let item_guid: u32 = self.world_context.next_item_guid();
                    let stack_count_to_add =
                        remaining_stack_count.min(item_template.max_stack_count);
                    let item = Item::new(
                        item_guid,
                        item_id,
                        self.guid.raw(),
                        stack_count_to_add,
                        false,
                    );
                    remaining_stack_count = remaining_stack_count - stack_count_to_add;

                    let packet = ServerMessage::new(SmsgCreateObject {
                        updates_count: 1,
                        has_transport: false,
                        updates: vec![item.build_create_data()],
                    });

                    self.inventory.set(slot, item);
                    self.session.send(&packet).unwrap();

                    chosen_slot = slot;
                }
                None => return Err(InventoryResult::InventoryFull),
            }
        }

        let total_count = self.inventory.get_item_count(item_id);
        let packet = ServerMessage::new(SmsgItemPushResult {
            player_guid: self.guid.clone(),
            loot_source: 0,
            is_created: 0,
            is_visible_in_chat: 1,
            bag_slot: 255, // FIXME: INVENTORY_SLOT_BAG_0
            item_slot: chosen_slot,
            item_id,
            item_suffix_factor: 0,      // FIXME
            item_random_property_id: 0, // FIXME
            count: stack_count,
            total_count_of_this_item_in_inventory: total_count,
        });
        self.session.send(&packet).unwrap();

        // Try to complete in-progress quests
        let quest_ids: Vec<u32> = self
            .quest_statuses
            .iter()
            .filter_map(|(&quest_id, context)| {
                if context.status == PlayerQuestStatus::InProgress {
                    Some(quest_id)
                } else {
                    None
                }
            })
            .collect();
        let world_context_local_clone = self.world_context.clone();
        for quest_id in quest_ids {
            let Some(&ref quest_template) = world_context_local_clone
                .data_store
                .get_quest_template(quest_id)
            else {
                continue;
            };

            self.try_complete_quest(&quest_template);
        }

        Ok(chosen_slot)
    }

    pub fn remove_item(&mut self, slot: u32) -> Option<Item> {
        self.inventory.remove(slot).or_else(|| {
            error!("Player::remove_item: no item found in slot {slot}");
            None
        })

        // TODO: recalculate quest status (potentially back from ObjectivesCompleted to InProgress)
    }

    pub fn get_inventory_item(&self, slot: u32) -> Option<&Item> {
        self.inventory.get(slot)
    }

    pub fn try_equip_item_from_inventory(&mut self, from_slot: u32) -> InventoryResult {
        let Some(item_to_equip) = self.inventory.get(from_slot) else {
            return InventoryResult::SlotIsEmpty;
        };

        let Some(item_template) = self
            .world_context
            .data_store
            .get_item_template(item_to_equip.entry())
        else {
            return InventoryResult::ItemNotFound;
        };

        let Some(destination_slot) = self.inventory.equipment_slot_for(item_template) else {
            return InventoryResult::ItemCantBeEquipped;
        };
        let destination_slot = destination_slot as u32;

        if self.inventory.has_item_in_slot(destination_slot) {
            self.inventory.swap(from_slot, destination_slot);
        } else {
            self.inventory.move_item(from_slot, destination_slot);
        }

        InventoryResult::Ok
    }

    pub fn try_swap_inventory_item(&mut self, from_slot: u32, to_slot: u32) -> InventoryResult {
        let (maybe_moved_item, maybe_target_item) = self.inventory.get2_mut(from_slot, to_slot);

        // There's no item in from_slot (cheating player?)
        let Some(moved_item) = maybe_moved_item else {
            return InventoryResult::SlotIsEmpty;
        };
        let moved_item_entry = moved_item.entry();
        let moved_item_stack_count = moved_item.stack_count();

        let Some(moved_item_template) = self
            .world_context
            .data_store
            .get_item_template(moved_item.entry())
        else {
            return InventoryResult::ItemNotFound;
        };

        let is_destination_gear_slot =
            to_slot >= InventorySlot::EQUIPMENT_START && to_slot < InventorySlot::EQUIPMENT_END;
        let is_origin_gear_slot =
            from_slot >= InventorySlot::EQUIPMENT_START && from_slot < InventorySlot::EQUIPMENT_END;

        // Equipment is dragged over a gear slot
        // => Check that moved_item can go in to_slot
        if is_destination_gear_slot {
            let allowed_gear_slots: Vec<u32> = moved_item_template
                .allowed_gear_slots()
                .into_iter()
                .map(|slot| slot as u32)
                .collect();
            if !allowed_gear_slots.contains(&to_slot) {
                return InventoryResult::ItemDoesntGoToSlot;
            }
        }

        if let Some(target_item) = maybe_target_item {
            let target_item_entry = target_item.entry();
            let target_item_stack_count = target_item.stack_count();
            let Some(target_item_template) = self
                .world_context
                .data_store
                .get_item_template(target_item.entry())
            else {
                return InventoryResult::ItemNotFound;
            };

            // Equipment is dragged from gear onto another gear piece in a bag
            // => Check that target_item can go in from_slot
            if is_origin_gear_slot {
                let allowed_gear_slots: Vec<u32> = target_item_template
                    .allowed_gear_slots()
                    .into_iter()
                    .map(|slot| slot as u32)
                    .collect();
                if !allowed_gear_slots.contains(&from_slot) {
                    return InventoryResult::ItemDoesntGoToSlot;
                }
            }

            // If both items are the same template and target still has available stack space,
            // recalculate both stacks
            if moved_item_entry == target_item_entry
                && target_item_stack_count < target_item_template.max_stack_count
            {
                let stack_diff = (target_item_template.max_stack_count - target_item_stack_count)
                    .min(moved_item_stack_count) as i32;

                target_item.change_stack_count(stack_diff);
                if stack_diff < moved_item_stack_count as i32 {
                    moved_item.change_stack_count(stack_diff as i32 * -1);
                } else {
                    // If the moved item has no stack after the transfer, delete it
                    self.remove_item(from_slot);
                }

                return InventoryResult::Ok;
            }

            self.inventory.swap(from_slot, to_slot);
            InventoryResult::Ok
        } else {
            if is_destination_gear_slot {
                // Moving the item from a bag to gear: equip it
                self.try_equip_item_from_inventory(from_slot)
            } else {
                // Moving the item to a bag (from gear or a bag): just move it
                self.inventory.move_item(from_slot, to_slot);
                InventoryResult::Ok
            }
        }
    }

    pub fn try_split_item(
        &mut self,
        from_slot: u32,
        destination_slot: u32,
        count: u8,
    ) -> InventoryResult {
        match self.inventory.get2_mut(from_slot, destination_slot) {
            (None, _) => {
                // There's no item in from_slot (cheating player?)
                InventoryResult::SlotIsEmpty
            }
            (Some(moved_item), _) if moved_item.stack_count() <= count.into() => {
                InventoryResult::CouldntSplitItems
            }
            (Some(moved_item), None) => {
                // Dropping the extra stacks on an empty slot
                moved_item.change_stack_count(count as i32 * -1);
                let new_item_guid: u32 = self.world_context.next_item_guid();
                let new_item = Item::new(
                    new_item_guid,
                    moved_item.entry(),
                    self.guid.raw(),
                    count.into(),
                    false,
                );
                let packet = ServerMessage::new(SmsgCreateObject {
                    updates_count: 1,
                    has_transport: false,
                    updates: vec![new_item.build_create_data()],
                });

                self.inventory.set(destination_slot, new_item);
                self.session.send(&packet).unwrap();

                InventoryResult::Ok
            }
            (Some(moved_item), Some(target_item)) if moved_item.entry() != target_item.entry() => {
                // Dropping the extra stacks on another item
                InventoryResult::CouldntSplitItems
            }
            (Some(moved_item), Some(target_item)) => {
                // Dropping the extra stacks on the same item
                let Some(target_item_template) = self
                    .world_context
                    .data_store
                    .get_item_template(target_item.entry())
                else {
                    return InventoryResult::ItemNotFound;
                };
                let available_stack_count =
                    target_item_template.max_stack_count - target_item.stack_count();
                let stacks_to_move = available_stack_count.min(count.into()) as i32;

                moved_item.change_stack_count(stacks_to_move * -1);
                target_item.change_stack_count(stacks_to_move);
                InventoryResult::Ok
            }
        }
    }

    pub fn inventory(&self) -> &PlayerInventory {
        &self.inventory
    }

    pub fn inventory_mut(&mut self) -> &mut PlayerInventory {
        &mut self.inventory
    }

    pub fn get_inventory_updates_and_reset(&mut self) -> Vec<UpdateData> {
        self.inventory.list_updated_and_reset()
    }
}
