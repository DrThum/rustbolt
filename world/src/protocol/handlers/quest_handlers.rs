use std::sync::Arc;

use log::{error, warn};
use shipyard::{Get, View, ViewMut};

use crate::datastore::data_types::QuestTemplate;
use crate::ecs::components::quest_actor::QuestActor;
use crate::entities::attributes::Attributes;
use crate::entities::object_guid::ObjectGuid;
use crate::entities::player::Player;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};
use crate::session::world_session::WorldSession;
use crate::shared::constants::PlayerQuestStatus;
use crate::DataStore;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_quest_giver_status_query(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
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

            if let Some(status) = maybe_status {
                let packet = ServerMessage::new(SmsgQuestGiverStatus {
                    guid: cmsg.guid,
                    status,
                });

                session.send(&packet).unwrap();
            };
        }
    }

    pub(crate) fn handle_cmsg_quest_giver_hello(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgQuestGiverHello = ClientMessage::read_as(data).unwrap();

        OpcodeHandler::send_initial_gossip_menu(cmsg.guid, session.clone(), world_context.clone());
    }

    pub(crate) fn handle_cmsg_quest_giver_query_quest(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgQuestGiverQueryQuest = ClientMessage::read_as(data).unwrap();

        if let Some(quest_template) = world_context.data_store.get_quest_template(cmsg.quest_id) {
            Self::send_quest_details(
                cmsg.guid,
                quest_template,
                session.clone(),
                world_context.data_store.clone(),
            );
        } else {
            error!(
                "received CMSG_QUESTGIVER_QUERY_QUEST for unknown quest {}",
                cmsg.quest_id
            );
        }
    }

    // Note: the quest giver can be a player in case of quest sharing
    pub(crate) fn handle_cmsg_quest_giver_accept_quest(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
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
        PacketHandlerArgs {
            session,
            world_context,
            ..
        }: PacketHandlerArgs,
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
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgQuestLogRemoveQuest = ClientMessage::read_as(data).unwrap();

        let map = session.current_map().unwrap();
        map.world().run(|mut vm_player: ViewMut<Player>| {
            vm_player[session.player_entity_id().unwrap()].remove_quest(cmsg.slot as usize);
        });
    }

    pub(crate) fn handle_cmsg_quest_giver_complete_quest(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
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

    pub(crate) fn handle_cmsg_quest_giver_request_reward(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgQuestGiverRequestReward = ClientMessage::read_as(data).unwrap();

        if let Some(map) = session.current_map() {
            map.world().run(|mut vm_player: ViewMut<Player>| {
                if let Ok(mut player) = (&mut vm_player).get(session.player_entity_id().unwrap()) {
                    if let Some(quest_template) =
                        world_context.data_store.get_quest_template(cmsg.quest_id)
                    {
                        player.try_complete_quest(quest_template);

                        if player.can_turn_in_quest(&cmsg.quest_id) {
                            let packet =
                                ServerMessage::new(SmsgQuestGiverOfferReward::from_template(
                                    cmsg.entity_guid,
                                    true,
                                    quest_template,
                                    world_context.data_store.clone(),
                                ));
                            session.send(&packet).unwrap();
                        }
                    }
                }
            });
        }
    }

    pub(crate) fn handle_cmsg_quest_giver_choose_reward(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgQuestGiverChooseReward = ClientMessage::read_as(data).unwrap();

        let map = session.current_map().unwrap();
        map.world().run(
            |mut vm_player: ViewMut<Player>,
             v_quest_actor: View<QuestActor>,
             mut vm_attributes: ViewMut<Attributes>| {
                let player = &mut vm_player[session.player_entity_id().unwrap()];
                let attributes =
                    &mut vm_attributes[session.player_entity_id().unwrap()];

                if player.can_turn_in_quest(&cmsg.quest_id) {
                    if let Some(gained_xp) = player.reward_quest(
                        cmsg.quest_id,
                        cmsg.chosen_reward_index,
                        world_context.data_store.clone(),
                        attributes,
                    ) {
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

                        if next_quest_id != 0
                            && v_quest_actor[quest_giver_entity_id].starts_quest(next_quest_id)
                        {
                            let next_quest = world_context
                                .data_store
                                .get_quest_template(next_quest_id)
                                .unwrap();

                            if player.can_start_quest(next_quest) {
                                Self::send_quest_details(
                                    cmsg.quest_giver_guid,
                                    next_quest,
                                    session.clone(),
                                    world_context.data_store.clone(),
                                );
                            }
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
        data_store: Arc<DataStore>,
    ) {
        let reward_choice_items = quest_template.reward_choice_items();
        let reward_items = quest_template.reward_items();

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
                    let item_display_id = data_store
                        .get_item_template(*id)
                        .map_or(0, |it| it.display_id);
                    QuestDetailsItemRewards {
                        item_id: *id,
                        item_count: *count,
                        item_display_id,
                    }
                })
                .collect(),
            reward_items_count: reward_items.len() as u32,
            reward_items: reward_items
                .iter()
                .map(|(id, count)| {
                    let item_display_id = data_store
                        .get_item_template(*id)
                        .map_or(0, |it| it.display_id);
                    QuestDetailsItemRewards {
                        item_id: *id,
                        item_count: *count,
                        item_display_id,
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
