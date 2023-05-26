use crate::shared::constants::ActionButtonType;

pub struct CharacterSkill {
    pub skill_id: u16,
    pub value: u16,
    pub max_value: u16,
}

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
