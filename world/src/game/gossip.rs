use crate::shared::constants::QuestGiverStatus;

#[derive(Debug)]
pub struct GossipMenu {
    pub menu_id: u32,
    pub title_text_id: u32,
    pub items: Vec<GossipMenuItem>,
    pub quests: Vec<GossipMenuQuestItem>,
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
}

#[derive(Debug)]
pub struct GossipMenuItem {
    pub icon: u8, // TODO: Enum?
    pub coded: bool,
    pub required_money: u32,
    pub message: String,
    pub box_message: String,
}

#[derive(Debug)]
pub struct GossipMenuQuestItem {
    pub quest_id: u32,
    pub icon: u32, // TODO: Enum?
                   // Also quest level and title but it must come from datastore
}
