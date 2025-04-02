use crate::{
    entities::position::WorldPosition,
    shared::constants::{ActionButtonType, PlayerQuestStatus, MAX_QUEST_OBJECTIVES_COUNT},
};

pub struct CharacterSkill {
    pub skill_id: u16,
    pub value: u16,
    pub max_value: u16,
}

#[derive(Clone)]
pub struct ActionButton {
    pub position: u32,
    pub action_type: ActionButtonType,
    pub action_value: u32,
}

impl ActionButton {
    pub fn packed(&self) -> u32 {
        self.action_value | ((self.action_type as u32) << 24)
    }
}

#[derive(Clone)]
pub struct FactionStanding {
    pub faction_id: u32,
    pub base_standing: i32,
    pub db_standing: i32,
    pub flags: u32,
    pub position_in_reputation_list: u32,
}

impl FactionStanding {
    #[allow(dead_code)]
    pub fn standing(&self) -> i32 {
        self.base_standing + self.db_standing
    }
}

#[derive(Debug, Clone, Copy)]
pub struct QuestLogContext {
    pub slot: Option<usize>,
    pub status: PlayerQuestStatus,
    pub entity_counts: [u32; MAX_QUEST_OBJECTIVES_COUNT],
}

#[derive(Clone, Copy)]
pub struct BindPoint {
    pub map_id: u32,
    pub area_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
}

impl BindPoint {
    pub fn from_position(position: &WorldPosition, area_id: u32) -> Self {
        Self {
            map_id: position.map_key.map_id,
            area_id,
            x: position.x,
            y: position.y,
            z: position.z,
            o: position.o,
        }
    }
}
