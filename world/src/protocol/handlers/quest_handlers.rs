use std::sync::Arc;

use log::{error, warn};
use shipyard::{Get, View, ViewMut};

use crate::ecs::components::quest_actor::QuestActor;
use crate::entities::object_guid::ObjectGuid;
use crate::entities::player::Player;
use crate::game::gossip::GossipMenu;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;
use crate::shared::constants::{PlayerQuestStatus, QuestGiverStatus};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_quest_giver_status_query(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverStatusQuery = ClientMessage::read_as(data).unwrap();

        if let Some(guid) = ObjectGuid::from_raw(cmsg.guid) {
            let map = session.current_map().unwrap();
            map.world()
                .run(|v_player: View<Player>, v_quest_actor: View<QuestActor>| {
                    let guid_entity_id = map.lookup_entity_ecs(&guid).unwrap();
                    if let Ok(quest_actor) = v_quest_actor.get(guid_entity_id) {
                        let status = quest_actor.quest_status_for_player(
                            v_player.get(session.player_entity_id().unwrap()).unwrap(),
                            world_context.clone(),
                        );

                        let packet = ServerMessage::new(SmsgQuestGiverStatus {
                            guid: cmsg.guid,
                            status,
                        });

                        session.send(&packet).unwrap();
                    }
                });
        }
    }

    pub(crate) fn handle_cmsg_quest_giver_hello(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverHello = ClientMessage::read_as(data).unwrap();

        if let Some(target_guid) = ObjectGuid::from_raw(cmsg.guid) {
            let map = session.current_map().unwrap();
            map.world()
                .run(|v_player: View<Player>, v_quest_actor: View<QuestActor>| {
                    let my_entity_id = session.player_entity_id().unwrap();
                    let player = &v_player[my_entity_id];

                    let quest_giver_entity_id = map.lookup_entity_ecs(&target_guid).unwrap();
                    let quest_actor = &v_quest_actor[quest_giver_entity_id];

                    let mut gossip_menu = GossipMenu::new(0, 1); // FIXME
                    for quest_id in quest_actor.quests_started() {
                        let quest_template = world_context
                            .data_store
                            .get_quest_template(*quest_id)
                            .unwrap();
                        match player.quest_status(quest_id).map(|ctx| ctx.status) {
                            None if player.can_start_quest(quest_template) => {
                                gossip_menu.add_quest(*quest_id, QuestGiverStatus::Available)
                            }
                            status => warn!("unhandled case (quests started - {:?})", status),
                        }
                    }

                    for quest_id in quest_actor.quests_ended() {
                        match player.quest_status(quest_id).map(|ctx| ctx.status) {
                            Some(PlayerQuestStatus::InProgress) => {
                                gossip_menu.add_quest(*quest_id, QuestGiverStatus::Incomplete);
                            }
                            Some(PlayerQuestStatus::ObjectivesCompleted) => {
                                gossip_menu.add_quest(*quest_id, QuestGiverStatus::Reward);
                            }
                            status => warn!("unhandled case (quests ended - {:?})", status),
                        }
                    }

                    let packet = ServerMessage::new(SmsgGossipMessage::from_gossip_menu(
                        &target_guid,
                        &gossip_menu,
                        world_context.data_store.clone(),
                    ));

                    session.send(&packet).unwrap();
                });
        }
    }

    pub(crate) fn handle_cmsg_quest_giver_query_quest(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverQueryQuest = ClientMessage::read_as(data).unwrap();

        if let Some(quest_template) = world_context.data_store.get_quest_template(cmsg.quest_id) {
            let reward_choice_items = quest_template.reward_choice_items();
            let reward_items = quest_template.reward_items();

            let packet = ServerMessage::new(SmsgQuestGiverQuestDetails {
                guid: cmsg.guid, // TODO: Validate this
                quest_id: cmsg.quest_id,
                title: quest_template
                    .title
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
                objectives: quest_template
                    .objectives
                    .as_ref()
                    .unwrap_or(&"".to_owned())
                    .clone()
                    .into(),
                auto_accept: false,
                suggested_players: quest_template.suggested_players,
                reward_choice_items_count: reward_choice_items.len() as u32,
                reward_choice_items: reward_choice_items
                    .iter()
                    .map(|(id, count)| {
                        QuestDetailsItemRewards {
                            item_id: *id,
                            item_count: *count,
                            item_display_id: 0, // TODO: actual display id
                        }
                    })
                    .collect(),
                reward_items_count: reward_items.len() as u32,
                reward_items: reward_items
                    .iter()
                    .map(|(id, count)| {
                        QuestDetailsItemRewards {
                            item_id: *id,
                            item_count: *count,
                            item_display_id: 0, // TODO: actual display id
                        }
                    })
                    .collect(),
                required_or_reward_money: quest_template.required_or_reward_money,
                honor_reward: quest_template.reward_honorable_kills * 10, // FIXME: depends on level
                reward_spell: quest_template.reward_spell,
                reward_spell_cast: quest_template.reward_spell_cast,
                reward_title_bit_index: 0, // TODO: bit_index from CharTitlesStore.dbc
                emotes: [
                    QuestDetailsEmote {
                        emote: quest_template.details_emote1,
                        delay: quest_template.details_emote_delay1,
                    },
                    QuestDetailsEmote {
                        emote: quest_template.details_emote2,
                        delay: quest_template.details_emote_delay2,
                    },
                    QuestDetailsEmote {
                        emote: quest_template.details_emote3,
                        delay: quest_template.details_emote_delay3,
                    },
                    QuestDetailsEmote {
                        emote: quest_template.details_emote4,
                        delay: quest_template.details_emote_delay4,
                    },
                ],
            });

            session.send(&packet).unwrap();
        } else {
            error!(
                "received CMSG_QUESTGIVER_QUERY_QUEST for unknown quest {}",
                cmsg.quest_id
            );
        }
    }

    // Note: the quest giver can be a player in case of quest sharing
    pub(crate) fn handle_cmsg_quest_giver_accept_quest(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverAcceptQuest = ClientMessage::read_as(data).unwrap();

        if let Some(quest_template) = world_context.data_store.get_quest_template(cmsg.quest_id) {
            let map = session.current_map().unwrap();
            map.world().run(|mut vm_player: ViewMut<Player>| {
                let player = &mut vm_player[session.player_entity_id().unwrap()];
                player.start_quest(quest_template);
            });
        }
    }

    pub(crate) fn handle_cmsg_quest_giver_status_multiple_query(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        _data: Vec<u8>,
    ) {
        let map = session.current_map().unwrap();
        let mut statuses: Vec<QuestGiverStatusMultipleEntry> = Vec::new();

        map.world()
            .run(|v_player: View<Player>, v_quest_actor: View<QuestActor>| {
                for guid in session.known_guids() {
                    let guid_entity_id = map.lookup_entity_ecs(&guid).unwrap();
                    if let Ok(quest_actor) = v_quest_actor.get(guid_entity_id) {
                        let status = quest_actor.quest_status_for_player(
                            v_player.get(session.player_entity_id().unwrap()).unwrap(),
                            world_context.clone(),
                        );

                        statuses.push(QuestGiverStatusMultipleEntry {
                            guid: guid.raw(),
                            status,
                        });
                    }
                }
            });

        let packet = ServerMessage::new(SmsgQuestGiverStatusMultiple {
            count: statuses.len() as u32,
            statuses,
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_quest_log_remove_quest(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestLogRemoveQuest = ClientMessage::read_as(data).unwrap();

        let map = session.current_map().unwrap();
        map.world().run(|mut vm_player: ViewMut<Player>| {
            vm_player[session.player_entity_id().unwrap()].remove_quest(cmsg.slot as usize);
        });
    }
}
