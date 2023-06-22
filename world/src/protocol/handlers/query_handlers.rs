use std::sync::Arc;

use binrw::NullString;
use log::error;

use crate::{
    game::world_context::WorldContext,
    protocol::{
        client::ClientMessage,
        packets::{
            CmsgCreatureQuery, CmsgNpcTextQuery, CmsgQuestQuery, NpcTextUpdateEmote,
            SmsgCreatureQueryResponse, SmsgCreatureQueryResponseUnknownTemplate, SmsgNpcTextUpdate,
            SmsgQuestQueryResponse,
        },
        server::ServerMessage,
    },
    session::{opcode_handler::OpcodeHandler, world_session::WorldSession},
};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_creature_query(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgCreatureQuery = ClientMessage::read_as(data).unwrap();

        if let Some(template) = world_context.data_store.get_creature_template(cmsg.entry) {
            let packet = ServerMessage::new(SmsgCreatureQueryResponse {
                entry: template.entry,
                name: NullString::from(template.name.clone()),
                name2: 0,
                name3: 0,
                name4: 0,
                sub_name: NullString::from(template.sub_name.clone().unwrap_or("".to_owned())),
                icon_name: NullString::from(template.icon_name.clone().unwrap_or("".to_owned())),
                type_flags: template.type_flags,
                type_id: template.type_id,
                family: template.family,
                rank: template.rank,
                unk: 0,
                pet_spell_data_id: template.pet_spell_data_id,
                model_ids: template.model_ids.clone(),
                health_multiplier: template.health_multiplier,
                power_multiplier: template.power_multiplier,
                racial_leader: template.racial_leader,
            });

            session.send(&packet).unwrap();
        } else {
            let packet = ServerMessage::new(SmsgCreatureQueryResponseUnknownTemplate {
                masked_entry: cmsg.entry | 0x80000000,
            });

            session.send(&packet).unwrap();
        }
    }

    // TODO: Hardcoded impl
    pub(crate) fn handle_cmsg_npc_text_query(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgNpcTextQuery = ClientMessage::read_as(data).unwrap();

        let packet = ServerMessage::new(SmsgNpcTextUpdate {
            text_id: cmsg.text_id,
            probability: 0.0,
            text0: "TODO TEXT 0".into(),
            text1: "TODO TEXT 1".into(),
            language: 0,
            emotes: [
                NpcTextUpdateEmote {
                    delay: 0,
                    emote_id: 0,
                },
                NpcTextUpdateEmote {
                    delay: 0,
                    emote_id: 0,
                },
                NpcTextUpdateEmote {
                    delay: 0,
                    emote_id: 0,
                },
            ],
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_quest_query(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestQuery = ClientMessage::read_as(data).unwrap();

        if let Some(quest_template) = world_context.data_store.get_quest_template(cmsg.quest_id) {
            fn transform_entity_id(entity_id: i32) -> u32 {
                if entity_id >= 0 {
                    entity_id as u32
                } else {
                    (-entity_id) as u32 & 0x80000000
                }
            }

            let packet = ServerMessage::new(SmsgQuestQueryResponse {
                quest_id: quest_template.entry,
                method: quest_template.method,
                quest_level: quest_template.level,
                zone_or_sort: quest_template.zone_or_sort,
                quest_type: quest_template.type_,
                suggested_player: quest_template.suggested_players,
                rep_objective_faction: quest_template.rep_objective_faction,
                rep_objective_value: quest_template.rep_objective_value,
                required_opposite_rep_faction: 0,
                required_opposite_rep_value: 0,
                next_quest_in_chain: quest_template.next_quest_in_chain,
                required_or_reward_money: quest_template.required_or_reward_money,
                reward_money_max_level: quest_template.reward_money_max_level,
                reward_spell: quest_template.reward_spell,
                reward_spell_cast: quest_template.reward_spell_cast,
                reward_honorable_kills: quest_template.reward_honorable_kills,
                source_item_id: quest_template.source_item_id,
                quest_flags: quest_template.flags.bits(),
                character_title_id: quest_template.character_title,
                reward_items_id: [
                    quest_template.reward_item_id1,
                    quest_template.reward_item_id2,
                    quest_template.reward_item_id3,
                    quest_template.reward_item_id4,
                ],
                reward_items_count: [
                    quest_template.reward_item_count1,
                    quest_template.reward_item_count2,
                    quest_template.reward_item_count3,
                    quest_template.reward_item_count4,
                ],
                reward_choice_items_id: [
                    quest_template.reward_choice_item_id1,
                    quest_template.reward_choice_item_id2,
                    quest_template.reward_choice_item_id3,
                    quest_template.reward_choice_item_id4,
                    quest_template.reward_choice_item_id5,
                    quest_template.reward_choice_item_id6,
                ],
                reward_choice_items_count: [
                    quest_template.reward_choice_item_count1,
                    quest_template.reward_choice_item_count2,
                    quest_template.reward_choice_item_count3,
                    quest_template.reward_choice_item_count4,
                    quest_template.reward_choice_item_count5,
                    quest_template.reward_choice_item_count6,
                ],
                point_map_id: quest_template.point_map_id,
                point_x: quest_template.point_x,
                point_y: quest_template.point_y,
                point_opt: quest_template.point_opt,
                title: quest_template
                    .title
                    .as_ref()
                    .unwrap_or(&"".to_owned())
                    .clone()
                    .into(),
                objectives: quest_template
                    .objectives
                    .as_ref()
                    .unwrap_or(&"".to_owned())
                    .clone()
                    .into(),
                details: quest_template
                    .details
                    .as_ref()
                    .unwrap_or(&"".to_owned())
                    .clone()
                    .into(),
                end_text: quest_template
                    .end_text
                    .as_ref()
                    .unwrap_or(&"".to_owned())
                    .clone()
                    .into(),
                required_entities_id: [
                    transform_entity_id(quest_template.required_entity_id1),
                    transform_entity_id(quest_template.required_entity_id2),
                    transform_entity_id(quest_template.required_entity_id3),
                    transform_entity_id(quest_template.required_entity_id4),
                ],
                required_entities_count: [
                    quest_template.required_entity_count1,
                    quest_template.required_entity_count2,
                    quest_template.required_entity_count3,
                    quest_template.required_entity_count4,
                ],
                required_items_id: [
                    quest_template.required_item_id1,
                    quest_template.required_item_id2,
                    quest_template.required_item_id3,
                    quest_template.required_item_id4,
                ],
                required_items_count: [
                    quest_template.required_item_count1,
                    quest_template.required_item_count2,
                    quest_template.required_item_count3,
                    quest_template.required_item_count4,
                ],
                objective_texts: [
                    quest_template
                        .objective_text1
                        .as_ref()
                        .unwrap_or(&"".to_owned())
                        .clone()
                        .into(),
                    quest_template
                        .objective_text2
                        .as_ref()
                        .unwrap_or(&"".to_owned())
                        .clone()
                        .into(),
                    quest_template
                        .objective_text3
                        .as_ref()
                        .unwrap_or(&"".to_owned())
                        .clone()
                        .into(),
                    quest_template
                        .objective_text4
                        .as_ref()
                        .unwrap_or(&"".to_owned())
                        .clone()
                        .into(),
                ],
            });

            session.send(&packet).unwrap();
        } else {
            error!(
                "received CMSG_QUEST_QUERY for unknown quest id {}",
                cmsg.quest_id
            );
        }
    }
}
