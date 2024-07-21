use std::sync::Arc;

use shipyard::Component;

use crate::{
    datastore::data_types::{QuestActorRole, QuestRelation},
    entities::player::Player,
    game::world_context::WorldContext,
    shared::constants::QuestGiverStatus,
};

#[derive(Component)]
pub struct QuestActor {
    starts: Vec<u32>,
    ends: Vec<u32>,
}

impl QuestActor {
    pub fn new(quest_relations: Option<&Vec<QuestRelation>>) -> Self {
        Self {
            starts: quest_relations
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|rel| {
                    if rel.role == QuestActorRole::Start {
                        Some(rel.quest_id)
                    } else {
                        None
                    }
                })
                .collect(),
            ends: quest_relations
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|rel| {
                    if rel.role == QuestActorRole::End {
                        Some(rel.quest_id)
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }

    pub fn quests_started(&self) -> &[u32] {
        &self.starts
    }

    pub fn starts_quest(&self, quest_id: u32) -> bool {
        self.starts.contains(&quest_id)
    }

    pub fn quests_ended(&self) -> &[u32] {
        &self.ends
    }

    pub fn quest_status_for_player(
        &self,
        player: &Player,
        world_context: Arc<WorldContext>,
    ) -> QuestGiverStatus {
        for quest_id in self.ends.iter() {
            if player.can_turn_in_quest(&quest_id) {
                return QuestGiverStatus::Reward;
            } else if player.is_progressing_quest(&quest_id) {
                return QuestGiverStatus::Incomplete;
            }
        }

        for quest_id in self.starts.iter() {
            let quest_template = world_context
                .data_store
                .get_quest_template(*quest_id)
                .unwrap();
            if player.can_start_quest(quest_template) {
                return QuestGiverStatus::Available;
            }
            // TODO: if player level is within 5 levels of quest level, return Unavailable (or
            // something like that)
        }

        return QuestGiverStatus::None;
    }
}
