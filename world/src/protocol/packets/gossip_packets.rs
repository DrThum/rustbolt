use std::sync::Arc;

use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

use crate::entities::object_guid::ObjectGuid;
use crate::game::gossip::GossipMenu;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::GossipMenuItemIcon;
use crate::DataStore;

#[binwrite]
#[server_opcode]
pub struct SmsgGossipMessage {
    source_guid: u64,
    menu_id: u32,
    title_text_id: u32,
    menu_items_count: u32,
    menu_items: Vec<SmsgGossipMessageMenuItem>,
    menu_quest_items_count: u32,
    menu_quest_items: Vec<SmsgGossipMessageMenuQuestItem>,
}

impl SmsgGossipMessage {
    pub fn from_gossip_menu(
        source_guid: &ObjectGuid,
        menu: &GossipMenu,
        data_store: Arc<DataStore>,
    ) -> Self {
        let menu_items: Vec<SmsgGossipMessageMenuItem> = menu
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| SmsgGossipMessageMenuItem {
                index: index as u32,
                icon: item.icon,
                coded: item.coded,
                required_money: item.required_money,
                message: item.message.clone().into(),
                box_message: item.box_message.clone().into(),
            })
            .collect();

        let menu_quest_items: Vec<SmsgGossipMessageMenuQuestItem> = menu
            .quests
            .iter()
            .map(|item| {
                let quest_template = data_store.get_quest_template(item.quest_id).unwrap();

                SmsgGossipMessageMenuQuestItem {
                    quest_id: item.quest_id,
                    icon: item.icon,
                    quest_level: quest_template.level,
                    title: quest_template
                        .title
                        .as_ref()
                        .unwrap_or(&"".to_owned())
                        .clone()
                        .into(),
                }
            })
            .collect();

        Self {
            source_guid: source_guid.raw(),
            menu_id: menu.menu_id,
            title_text_id: menu.title_text_id,
            menu_items_count: menu.items.len() as u32,
            menu_items,
            menu_quest_items_count: menu.quests.len() as u32,
            menu_quest_items,
        }
    }
}

#[binwrite]
struct SmsgGossipMessageMenuItem {
    pub index: u32,
    #[bw(map = |gmii: &GossipMenuItemIcon| *gmii as u8)]
    pub icon: GossipMenuItemIcon,
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub coded: bool,
    pub required_money: u32,
    pub message: NullString,
    pub box_message: NullString,
}

#[binwrite]
struct SmsgGossipMessageMenuQuestItem {
    pub quest_id: u32,
    pub icon: u32,
    pub quest_level: i32,
    pub title: NullString,
}

#[binread]
pub struct CmsgGossipHello {
    pub guid: u64,
}

#[binread]
pub struct CmsgGossipSelectOption {
    pub guid: ObjectGuid,
    pub menu_id: u32,
    pub option_index: u32,
    // TODO: read a string here if the chosen option has box_coded == true
}
