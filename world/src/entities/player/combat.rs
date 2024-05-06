use std::time::Duration;
use std::{collections::HashSet, sync::Arc};

use shared::utils::value_range::ValueRange;

use crate::shared::constants::{BASE_ATTACK_TIME, BASE_DAMAGE};
use crate::{
    datastore::data_types::QuestTemplate,
    entities::{
        internal_values::{QuestSlotOffset, QUEST_SLOT_OFFSETS_COUNT},
        object_guid::ObjectGuid,
    },
    protocol::{packets::SmsgQuestUpdateAddKill, server::ServerMessage},
    shared::constants::{InventorySlot, PlayerQuestStatus, WeaponAttackType},
    DataStore,
};

use super::{Player, UnitFields};

impl Player {
    pub fn set_in_combat_with(&self, guid: ObjectGuid) {
        self.in_combat_with.write().insert(guid);
    }

    pub fn reset_in_combat_with(&self) {
        self.in_combat_with.write().clear();
    }

    pub fn unset_in_combat_with(&self, guid: ObjectGuid) {
        self.in_combat_with.write().remove(&guid);
    }

    pub fn in_combat_with(&self) -> HashSet<ObjectGuid> {
        self.in_combat_with.read().clone()
    }

    pub fn is_in_combat_with(&self, other: &ObjectGuid) -> bool {
        self.in_combat_with.read().contains(other)
    }

    pub fn base_attack_time(
        &self,
        attack_type: WeaponAttackType,
        data_store: Arc<DataStore>,
    ) -> Duration {
        let slot = match attack_type {
            WeaponAttackType::MainHand => InventorySlot::EquipmentMainHand,
            WeaponAttackType::OffHand => InventorySlot::EquipmentOffHand,
            WeaponAttackType::Ranged => InventorySlot::EquipmentRanged,
        } as u32;

        self.inventory
            .get(slot)
            .and_then(|item| {
                data_store
                    .get_item_template(item.entry())
                    .map(|template| Duration::from_millis(template.delay as u64))
            })
            .unwrap_or(BASE_ATTACK_TIME)
    }

    pub fn base_damage(
        &self,
        attack_type: WeaponAttackType,
        data_store: Arc<DataStore>,
    ) -> ValueRange<f32> {
        let slot = match attack_type {
            WeaponAttackType::MainHand => InventorySlot::EquipmentMainHand,
            WeaponAttackType::OffHand => InventorySlot::EquipmentOffHand,
            WeaponAttackType::Ranged => InventorySlot::EquipmentRanged,
        } as u32;

        self.inventory
            .get(slot)
            .and_then(|item| {
                data_store.get_item_template(item.entry()).map(|template| {
                    let min = template
                        .damages
                        .iter()
                        .map(|dmg| dmg.damage_min)
                        .sum::<f32>();

                    let max = template
                        .damages
                        .iter()
                        .map(|dmg| dmg.damage_max)
                        .sum::<f32>();

                    ValueRange::new(min, max)
                })
            })
            .unwrap_or(ValueRange::new(BASE_DAMAGE, BASE_DAMAGE))
    }

    pub fn notify_killed_creature(&mut self, creature_guid: &ObjectGuid, creature_entry: u32) {
        // Update quest kills counters
        let mut updated_quests: Vec<QuestTemplate> = Vec::new();
        self.quest_statuses.iter_mut().for_each(|(quest_id, ctx)| {
            let quest_template = self
                .world_context
                .data_store
                .get_quest_template(*quest_id)
                .expect("player has non-existing quest in log");

            if let Some((objective_index, required_count)) =
                quest_template.creature_requirements(creature_entry)
            {
                match (ctx.status, ctx.slot) {
                    (PlayerQuestStatus::InProgress, Some(slot)) => {
                        let current_count = ctx.entity_counts[objective_index];
                        if current_count < required_count {
                            let new_count = (current_count + 1).min(required_count);
                            ctx.entity_counts[objective_index] = new_count;

                            {
                                let mut values_guard = self.internal_values.write();
                                let index = UnitFields::PlayerQuestLog1_1 as usize
                                    + (slot * QUEST_SLOT_OFFSETS_COUNT
                                        + QuestSlotOffset::Counters as usize);

                                values_guard.set_u8(index, objective_index, new_count as u8);
                            }

                            let packet = ServerMessage::new(SmsgQuestUpdateAddKill {
                                quest_id: quest_template.entry,
                                entity_id: creature_entry,
                                new_count,
                                required_count,
                                guid: creature_guid.raw(),
                            });

                            self.session.send(&packet).unwrap();

                            updated_quests.push(quest_template.clone());
                        }
                    }
                    _ => (),
                }
            }
        });

        // Try to complete the affected quests
        for quest_template in updated_quests {
            self.try_complete_quest(&quest_template);
        }
    }
}
