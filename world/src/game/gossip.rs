use crate::{datastore::data_types::GossipMenuOption, shared::constants::QuestGiverStatus};

#[derive(Debug, Clone)]
pub struct GossipMenu {
    pub menu_id: u32,
    pub title_text_id: u32,
    pub items: Vec<GossipMenuItem>,
    pub quests: Vec<GossipMenuQuestItem>,
}

impl Default for GossipMenu {
    fn default() -> Self {
        Self {
            menu_id: 0,
            title_text_id: 1, // "Greetings $N"
            items: Vec::new(),
            quests: Vec::new(),
        }
    }
}

impl GossipMenu {
    pub fn new(menu_id: u32, title_text_id: u32) -> Self {
        Self {
            menu_id,
            title_text_id,
            items: Vec::new(),
            quests: Vec::new(),
        }
    }

    pub fn add_quest(&mut self, quest_id: u32, icon: QuestGiverStatus) {
        self.quests.push(GossipMenuQuestItem {
            quest_id,
            icon: icon as u32,
        });
    }

    pub fn add_item(&mut self, option: &GossipMenuOption) {
        self.items.push(GossipMenuItem {
            icon: option.icon as u8,
            coded: option.box_coded,
            required_money: option.box_money,
            message: option.text.as_ref().cloned().unwrap_or_default(),
            box_message: option.box_text.as_ref().cloned().unwrap_or_default(),
        });
    }
}

#[derive(Debug, Clone)]
pub struct GossipMenuItem {
    pub icon: u8, // TODO: Enum?
    pub coded: bool,
    pub required_money: u32,
    pub message: String,
    pub box_message: String,
}

#[derive(Debug, Clone)]
pub struct GossipMenuQuestItem {
    pub quest_id: u32,
    pub icon: u32, // TODO: Enum?
                   // Also quest level and title but it must come from datastore
}
