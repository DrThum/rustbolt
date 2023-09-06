use std::sync::Arc;

use log::{error, warn};
use shipyard::{Get, View, ViewMut};

use crate::datastore::data_types::QuestTemplate;
use crate::ecs::components::quest_actor::QuestActor;
use crate::entities::object_guid::ObjectGuid;
use crate::entities::player::Player;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;
use crate::shared::constants::PlayerQuestStatus;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_quest_giver_status_query(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverStatusQuery = ClientMessage::read_as(data).unwrap();

        if let Some(guid) = ObjectGuid::from_raw(cmsg.guid) {
            let map = session.current_map().unwrap();
            let maybe_status =
                map.world()
                    .run(|v_player: View<Player>, v_quest_actor: View<QuestActor>| {
                        let guid_entity_id = map.lookup_entity_ecs(&guid).unwrap();
                        v_quest_actor.get(guid_entity_id).ok().map(|quest_actor| {
                            quest_actor.quest_status_for_player(
                                v_player.get(session.player_entity_id().unwrap()).unwrap(),
                                world_context.clone(),
                            )
                        })
                    });

            maybe_status.map(|status| {
                let packet = ServerMessage::new(SmsgQuestGiverStatus {
                    guid: cmsg.guid,
                    status,
                });

                session.send(&packet).unwrap();
            });
        }
    }

    pub(crate) fn handle_cmsg_quest_giver_hello(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverHello = ClientMessage::read_as(data).unwrap();

        OpcodeHandler::send_initial_gossip_menu(cmsg.guid, session.clone(), world_context.clone());
    }

    pub(crate) fn handle_cmsg_quest_giver_query_quest(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverQueryQuest = ClientMessage::read_as(data).unwrap();

        if let Some(quest_template) = world_context.data_store.get_quest_template(cmsg.quest_id) {
            Self::send_quest_details(cmsg.guid, quest_template, session.clone());
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

    pub(crate) fn handle_cmsg_quest_giver_complete_quest(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverCompleteQuest = ClientMessage::read_as(data).unwrap();

        let map = session.current_map().unwrap();
        let player_entity_id = session.player_entity_id().unwrap();

        if let Some(quest_template) = world_context.data_store.get_quest_template(cmsg.quest_id) {
            map.world().run(|v_player: View<Player>| {
                use PlayerQuestStatus::*;

                let status = v_player[player_entity_id]
                    .quest_status(&cmsg.quest_id)
                    .map(|ctx| ctx.status);

                match status {
                    Some(InProgress) | Some(ObjectivesCompleted) => {
                        if quest_template.request_items_text.is_some() {
                            let packet =
                                ServerMessage::new(SmsgQuestGiverRequestItems::from_template(
                                    cmsg.guid,
                                    &status.unwrap(),
                                    false,
                                    quest_template,
                                    world_context.data_store.clone(),
                                ));
                            session.send(&packet).unwrap();
                        } else {
                            let packet =
                                ServerMessage::new(SmsgQuestGiverOfferReward::from_template(
                                    cmsg.guid,
                                    false,
                                    quest_template,
                                    world_context.data_store.clone(),
                                ));
                            session.send(&packet).unwrap();
                        }
                    }
                    Some(status) => warn!(
                        "CMSG_QUESTGIVER_COMPLETE_QUEST called with unexpected status {status:?}"
                    ),
                    None => warn!(
                        "CMSG_QUESTGIVER_COMPLETE_QUEST called but player does not have the quest"
                    ),
                }
            });
        }
    }

    pub(crate) fn handle_cmsg_quest_giver_choose_reward(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverChooseReward = ClientMessage::read_as(data).unwrap();

        let map = session.current_map().unwrap();
        map.world().run(
            |mut vm_player: ViewMut<Player>, v_quest_actor: View<QuestActor>| {
                let player = &mut vm_player[session.player_entity_id().unwrap()];

                if player.can_turn_in_quest(&cmsg.quest_id) {
                    if let Some(gained_xp) =
                        player.reward_quest(cmsg.quest_id, world_context.data_store.clone())
                    {
                        let quest_template = world_context
                            .data_store
                            .get_quest_template(cmsg.quest_id)
                            .expect("attempt to reward a non-existing quest");
                        let packet = ServerMessage::new(SmsgQuestGiverQuestComplete {
                            quest_id: cmsg.quest_id,
                            unk: 0x03,
                            xp: gained_xp,
                            required_or_reward_money: quest_template.required_or_reward_money,
                            honorable_kills: 10 * quest_template.reward_honorable_kills,
                            reward_items_count: 0, // TODO: depends on what the player chose as a reward
                            reward_items: Vec::new(),
                        });

                        session.send(&packet).unwrap();

                        let quest_giver_entity_id = map
                            .lookup_entity_ecs(
                                &ObjectGuid::from_raw(cmsg.quest_giver_guid).unwrap(),
                            )
                            .unwrap();
                        let next_quest_id = quest_template.next_quest_in_chain;
                        let next_quest = world_context
                            .data_store
                            .get_quest_template(next_quest_id)
                            .unwrap();

                        if next_quest_id != 0
                            && v_quest_actor[quest_giver_entity_id].starts_quest(next_quest_id)
                            && player.can_start_quest(&next_quest)
                        {
                            Self::send_quest_details(
                                cmsg.quest_giver_guid,
                                next_quest,
                                session.clone(),
                            );
                        }
                    }
                } else {
                    warn!("unexpected player quest status in CMSG_QUESTGIVER_CHOOSE_REWARD");
                }
            },
        );
    }

    fn send_quest_details(
        quest_giver_guid: u64,
        quest_template: &QuestTemplate,
        session: Arc<WorldSession>,
    ) {
        let reward_choice_items = quest_template.reward_choice_items();
        let reward_items = quest_template.reward_items();

        // TODO: Refactor this and the other place it is used
        let packet = ServerMessage::new(SmsgQuestGiverQuestDetails {
            guid: quest_giver_guid, // TODO: Validate this
            quest_id: quest_template.entry,
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
    }
}
