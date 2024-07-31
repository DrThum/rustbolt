use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use log::{error, warn};

use crate::{
    datastore::data_types::QuestTemplate,
    entities::internal_values::{QuestSlotOffset, QUEST_SLOT_OFFSETS_COUNT},
    shared::constants::{
        CharacterClassBit, CharacterRaceBit, PlayerQuestStatus, QuestSlotState, QuestStartError,
        MAX_QUESTS_IN_LOG, MAX_QUEST_OBJECTIVES_COUNT,
    },
    DataStore,
};

use super::{player_data::QuestLogContext, Player, UnitFields};

impl Player {
    pub fn can_start_quest(&self, quest_template: &QuestTemplate) -> bool {
        self.check_quest_requirements(quest_template).is_none()
    }

    /**
     * Checks to perform:
     *
     * - (1) player does not have the quest (OK)
     * - (2) satisfy exclusive_group requirements (TODO)
     * - (3) player class is in required_classes mask (OK)
     * - (4) player race is in required_races mask (OK)
     * - (5) player level >= quest_template.min_level (OK)
     * - (6) player skill level >= quest_template required skill (TODO)
     * - (7) player reputation >= quest_template required reputation (TODO)
     * - (8) player has done the previous quest (previous_quest_id > 0) (OK)
     * - (9) player is actively doing the parent quest (previous_quest_id < 0) (TODO)
     * - (10) player can only have one timed quest at the same time (TODO)
     * - (11) player is not doing/has not done the next quest in chain (TODO)
     * - (12) player has done the previous quest in chain (see qInfo.prevChainQuests in MaNGOS) (TODO)
     * - (13) player still has daily quests allowance if quest is daily (TODO)
     * - (14) game event must be active if quest is related to one (TODO)
     */
    fn check_quest_requirements(&self, quest_template: &QuestTemplate) -> Option<QuestStartError> {
        // (1)
        if self.quest_statuses.contains_key(&quest_template.entry) {
            return Some(QuestStartError::AlreadyOn);
        }

        {
            let values = self.internal_values.read();

            // (3)
            let class_id = values.get_u8(UnitFields::UnitFieldBytes0.into(), 1);
            if !quest_template.required_classes.is_empty()
                && !quest_template
                    .required_classes
                    .contains(CharacterClassBit::from(class_id))
            {
                return Some(QuestStartError::FailedRequirement);
            }

            // (4)
            let race_id = values.get_u8(UnitFields::UnitFieldBytes0.into(), 0);
            if !quest_template.required_races.is_empty()
                && !quest_template
                    .required_races
                    .contains(CharacterRaceBit::from(race_id))
            {
                return Some(QuestStartError::WrongRace);
            }

            // (5)
            if values.get_u32(UnitFields::UnitFieldLevel.into()) < quest_template.min_level {
                return Some(QuestStartError::TooLowLevel);
            }
        }

        // (8)
        if let Some(previous_quest) = quest_template.previous_quest_id() {
            match self.quest_statuses.get(&previous_quest) {
                Some(context) if context.status == PlayerQuestStatus::TurnedIn => (),
                _ => return Some(QuestStartError::FailedRequirement),
            }
        }

        None
    }

    pub fn quest_status(&self, quest_id: &u32) -> Option<&QuestLogContext> {
        self.quest_statuses.get(quest_id)
    }

    pub fn quest_statuses(&self) -> &HashMap<u32, QuestLogContext> {
        &self.quest_statuses
    }

    pub fn get_active_quest_ids(&self) -> Vec<u32> {
        let mut active_quests: Vec<u32> = Vec::new();

        let values_guard = self.internal_values.read();
        for i in 0..MAX_QUESTS_IN_LOG {
            active_quests.push(
                values_guard.get_u32(
                    UnitFields::PlayerQuestLog1_1 as usize + (i * QUEST_SLOT_OFFSETS_COUNT),
                ),
            );
        }

        active_quests
    }

    pub fn can_turn_in_quest(&self, quest_id: &u32) -> bool {
        self.quest_status(quest_id)
            .is_some_and(|ctx| ctx.status == PlayerQuestStatus::ObjectivesCompleted)
    }

    pub fn is_progressing_quest(&self, quest_id: &u32) -> bool {
        self.quest_status(quest_id)
            .is_some_and(|ctx| ctx.status == PlayerQuestStatus::InProgress)
    }

    pub fn start_quest(&mut self, quest_template: &QuestTemplate) {
        if !self.can_start_quest(quest_template) {
            error!("attempt to start a quest that the player cannot start");
            return;
        }

        {
            let mut first_empty_slot: Option<usize> = None;
            let mut values_guard = self.internal_values.write();
            for i in 0..MAX_QUESTS_IN_LOG {
                let quest_id_in_slots = values_guard.get_u32(
                    UnitFields::PlayerQuestLog1_1 as usize + (i * QUEST_SLOT_OFFSETS_COUNT),
                );
                if quest_id_in_slots == 0 {
                    first_empty_slot = Some(i);
                    break;
                }
            }

            if let Some(slot) = first_empty_slot {
                let base_index =
                    UnitFields::PlayerQuestLog1_1 as usize + (slot * QUEST_SLOT_OFFSETS_COUNT);

                values_guard.set_u32(base_index, quest_template.entry);

                if let Some(timer) = quest_template
                    .time_limit
                    .filter(|limit| *limit != Duration::ZERO)
                {
                    values_guard.set_u32(
                        base_index + QuestSlotOffset::Timer as usize,
                        (SystemTime::now() + timer)
                            .duration_since(UNIX_EPOCH)
                            .expect("time went backward")
                            .as_millis() as u32,
                    );
                }

                let quest_log_context = QuestLogContext {
                    slot: Some(slot),
                    status: PlayerQuestStatus::InProgress,
                    entity_counts: [0, 0, 0, 0],
                };
                self.quest_statuses
                    .insert(quest_template.entry, quest_log_context);
            } else {
                error!("player quest log is full");
                return;
            }
        }

        self.session.force_refresh_nearby_game_objects(self);
        self.try_complete_quest(quest_template);
    }

    pub fn remove_quest(&mut self, slot_to_remove: usize) {
        self.quest_statuses.retain(|_, context| match context.slot {
            None => true,
            Some(slot) => slot != slot_to_remove,
        });

        let mut values_guard = self.internal_values.write();
        let base_index =
            UnitFields::PlayerQuestLog1_1 as usize + (slot_to_remove * QUEST_SLOT_OFFSETS_COUNT);
        for index in 0..QUEST_SLOT_OFFSETS_COUNT {
            values_guard.set_u32(base_index + index, 0);
        }
        drop(values_guard);

        self.session.force_refresh_nearby_game_objects(self);
    }

    pub fn try_complete_quest(&mut self, quest_template: &QuestTemplate) {
        let quest_id = quest_template.entry;
        if let Some(context) = self.quest_statuses.get_mut(&quest_id) {
            if context.status != PlayerQuestStatus::InProgress || context.slot.is_none() {
                return;
            }

            for index in 0..MAX_QUEST_OBJECTIVES_COUNT {
                let current_entity_count = context.entity_counts[index];
                let objective_entity_count = quest_template.required_entity_counts[index];

                if current_entity_count < objective_entity_count {
                    return;
                }

                let required_item_id = quest_template.required_item_ids[index];
                let required_item_count = quest_template.required_item_counts[index];

                if self.inventory.get_item_count(required_item_id) < required_item_count {
                    return;
                }
            }

            // TODO: Check exploration etc

            context.status = PlayerQuestStatus::ObjectivesCompleted;
            let mut values_guard = self.internal_values.write();
            let base_index = UnitFields::PlayerQuestLog1_1 as usize
                + (context.slot.unwrap() * QUEST_SLOT_OFFSETS_COUNT);
            values_guard.set_u32(
                base_index + QuestSlotOffset::State as usize,
                QuestSlotState::Completed as u32,
            );
            drop(values_guard);

            self.session.force_refresh_nearby_game_objects(self);
        }
    }

    pub fn reward_quest(
        &mut self,
        quest_id: u32,
        chosen_reward_index: u32,
        data_store: Arc<DataStore>,
    ) -> Option<u32> {
        warn!("TODO: Implement Player::reward_quest (reputation, ...)");

        if let Some(context) = self.quest_statuses.get_mut(&quest_id) {
            if let Some(quest_template) = data_store.get_quest_template(quest_id) {
                if context.status != PlayerQuestStatus::ObjectivesCompleted {
                    error!(
                        "attempt to reward a quest with an unexpected status {:?}",
                        context.status
                    );
                    return None;
                }

                context.status = PlayerQuestStatus::TurnedIn;

                {
                    let mut values_guard = self.internal_values.write();
                    let base_index = UnitFields::PlayerQuestLog1_1 as usize
                        + (context.slot.unwrap() * QUEST_SLOT_OFFSETS_COUNT);

                    for index in 0..QUEST_SLOT_OFFSETS_COUNT {
                        values_guard.set_u32(base_index + index, 0);
                    }
                }

                context.slot = None;

                // Disable nearby GameObjects that depend on that quest
                self.session.force_refresh_nearby_game_objects(self);

                // Take required items
                for (id, count) in quest_template.required_items() {
                    self.inventory.remove_item_count(id, count);
                }

                self.modify_money(quest_template.required_or_reward_money);

                match quest_template.reward_choice_items()[chosen_reward_index as usize] {
                    (0, _) | (_, 0) => (),
                    (id, count) => {
                        self.auto_store_new_item(id, count).unwrap();
                    }
                }

                for (id, count) in quest_template
                    .reward_items()
                    .into_iter()
                    .filter(|(id, count)| *id != 0 && *count != 0)
                {
                    self.auto_store_new_item(id, count).unwrap();
                }

                let xp = quest_template.experience_reward_at_level(self.level());
                self.give_experience(xp, None);
                return Some(xp);
            }
        }

        error!("attempt to reward an non-existing quest");
        None
    }

    pub fn get_all_needed_item_ids_for_quests(&self) -> HashSet<u32> {
        let mut item_ids: HashSet<u32> = HashSet::new();
        self.quest_statuses.iter().for_each(|(&quest_id, context)| {
            let Some(quest_template) = self.world_context.data_store.get_quest_template(quest_id)
            else {
                return;
            };

            // If the player still needs an item for a quest, add its ID to the list
            // If the quest is done or if the player has all of the items, don't
            if context.status == PlayerQuestStatus::InProgress {
                for i in 0..MAX_QUEST_OBJECTIVES_COUNT {
                    if context.entity_counts[i] < quest_template.required_item_counts[i] {
                        item_ids.insert(quest_template.required_item_ids[i]);
                    }
                }
            }
        });

        item_ids
    }
}
