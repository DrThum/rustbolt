use log::{error, trace, warn};
use shipyard::{Get, View};
use std::{collections::HashMap, sync::Arc};

use crate::{
    ecs::components::quest_actor::QuestActor,
    entities::{creature::Creature, object_guid::ObjectGuid, player::Player},
    game::{gossip::GossipMenu, world_context::WorldContext},
    protocol::{opcodes::Opcode, packets::SmsgGossipMessage, server::ServerMessage},
    shared::constants::{PlayerQuestStatus, QuestGiverStatus},
};

use super::world_session::WorldSession;

pub struct PacketHandlerArgs {
    pub session: Arc<WorldSession>,
    pub world_context: Arc<WorldContext>,
    pub data: Vec<u8>,
}
pub type PacketHandler = Box<dyn Send + Sync + Fn(PacketHandlerArgs)>;

macro_rules! define_handler {
    ($opcode:expr, $processing_mode:expr, $handler:expr) => {
        (
            $opcode as u32,
            (
                $processing_mode,
                Box::new(|args| $handler(args)) as PacketHandler,
            ),
        )
    };
}

macro_rules! define_movement_handler {
    ($opcode:expr) => {
        (
            $opcode as u32,
            (
                OpcodeProcessingMode::ProcessInMap,
                Box::new(|args| OpcodeHandler::handle_movement_packet($opcode)(args))
                    as PacketHandler,
            ),
        )
    };
}

pub struct OpcodeHandler {
    handlers: HashMap<u32, (OpcodeProcessingMode, PacketHandler)>,
}

impl Default for OpcodeHandler {
    fn default() -> Self {
        Self::new()
    }
}

pub enum OpcodeProcessingMode {
    ProcessInMap,
    ProcessImmediately,
    Ignore,
}

impl OpcodeHandler {
    pub fn new() -> Self {
        use OpcodeProcessingMode::*;

        Self {
            handlers: HashMap::from([
                define_handler!(Opcode::MsgNullAction, Ignore, OpcodeHandler::unhandled),
                define_handler!(
                    Opcode::CmsgCharCreate,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_char_create
                ),
                define_handler!(
                    Opcode::CmsgCharEnum,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_char_enum
                ),
                define_handler!(
                    Opcode::CmsgCharDelete,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_char_delete
                ),
                define_handler!(
                    Opcode::CmsgPlayerLogin,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_player_login
                ),
                define_handler!(
                    Opcode::CmsgPing,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_ping
                ),
                define_handler!(
                    Opcode::CmsgRealmSplit,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_realm_split
                ),
                define_handler!(
                    Opcode::CmsgLogoutRequest,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_logout_request
                ),
                define_handler!(
                    Opcode::CmsgItemQuerySingle,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_item_query_single
                ),
                define_handler!(
                    Opcode::CmsgNameQuery,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_name_query
                ),
                define_handler!(
                    Opcode::CmsgQueryTime,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_query_time
                ),
                define_handler!(
                    Opcode::CmsgUpdateAccountData,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_update_account_data
                ),
                define_handler!(
                    Opcode::CmsgTimeSyncResp,
                    ProcessImmediately,
                    OpcodeHandler::handle_time_sync_resp
                ),
                define_movement_handler!(Opcode::MsgMoveStartForward),
                define_movement_handler!(Opcode::MsgMoveStartBackward),
                define_movement_handler!(Opcode::MsgMoveStop),
                define_movement_handler!(Opcode::MsgMoveStartStrafeLeft),
                define_movement_handler!(Opcode::MsgMoveStartStrafeRight),
                define_movement_handler!(Opcode::MsgMoveStopStrafe),
                define_movement_handler!(Opcode::MsgMoveJump),
                define_movement_handler!(Opcode::MsgMoveStartTurnLeft),
                define_movement_handler!(Opcode::MsgMoveStartTurnRight),
                define_movement_handler!(Opcode::MsgMoveStopTurn),
                define_movement_handler!(Opcode::MsgMoveStartPitchUp),
                define_movement_handler!(Opcode::MsgMoveStartPitchDown),
                define_movement_handler!(Opcode::MsgMoveStopPitch),
                define_movement_handler!(Opcode::MsgMoveSetRunMode),
                define_movement_handler!(Opcode::MsgMoveSetWalkMode),
                define_movement_handler!(Opcode::MsgMoveFallLand),
                define_movement_handler!(Opcode::MsgMoveStartSwim),
                define_movement_handler!(Opcode::MsgMoveStopSwim),
                define_movement_handler!(Opcode::MsgMoveSetFacing),
                define_movement_handler!(Opcode::MsgMoveSetPitch),
                define_movement_handler!(Opcode::MsgMoveHeartbeat),
                define_movement_handler!(Opcode::CmsgMoveFallReset),
                define_movement_handler!(Opcode::CmsgMoveSetFly),
                define_movement_handler!(Opcode::MsgMoveStartAscend),
                define_movement_handler!(Opcode::MsgMoveStopAscend),
                define_movement_handler!(Opcode::CmsgMoveChngTransport),
                define_movement_handler!(Opcode::MsgMoveStartDescend),
                define_handler!(
                    Opcode::CmsgStandStateChange,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_stand_state_change
                ),
                define_handler!(
                    Opcode::CmsgSetSheathed,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_set_sheathed
                ),
                define_handler!(
                    Opcode::CmsgSetActiveVoiceChannel,
                    Ignore,
                    OpcodeHandler::unhandled
                ),
                define_handler!(
                    Opcode::CmsgMessageChat,
                    ProcessImmediately, // TODO: This should actually be process immediately first then
                    // forwarded to the map if it's a local message (say, yell, ...)
                    OpcodeHandler::handle_cmsg_message_chat
                ),
                define_handler!(
                    Opcode::CmsgTextEmote,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_text_emote
                ),
                define_handler!(
                    Opcode::CmsgCreatureQuery,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_creature_query
                ),
                define_handler!(
                    Opcode::CmsgAttackSwing,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_attack_swing
                ),
                define_handler!(
                    Opcode::CmsgSetSelection,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_set_selection
                ),
                define_handler!(
                    Opcode::CmsgAttackStop,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_attack_stop
                ),
                define_handler!(
                    Opcode::CmsgCastSpell,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_cast_spell
                ),
                define_handler!(
                    Opcode::CmsgCancelCast,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_cancel_cast
                ),
                define_handler!(
                    Opcode::CmsgQuestGiverStatusQuery,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_quest_giver_status_query
                ),
                define_handler!(
                    Opcode::CmsgQuestGiverHello,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_quest_giver_hello
                ),
                define_handler!(
                    Opcode::CmsgNpcTextQuery,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_npc_text_query
                ),
                define_handler!(
                    Opcode::CmsgQuestGiverQueryQuest,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_quest_giver_query_quest
                ),
                define_handler!(
                    Opcode::CmsgQuestGiverAcceptQuest,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_quest_giver_accept_quest
                ),
                define_handler!(
                    Opcode::CmsgQuestGiverStatusMultipleQuery,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_quest_giver_status_multiple_query
                ),
                define_handler!(
                    Opcode::CmsgQuestQuery,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_quest_query
                ),
                define_handler!(
                    Opcode::CmsgQuestLogRemoveQuest,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_quest_log_remove_quest
                ),
                define_handler!(
                    Opcode::CmsgGossipHello,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_gossip_hello
                ),
                define_handler!(
                    Opcode::CmsgGossipSelectOption,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_gossip_select_option
                ),
                define_handler!(
                    Opcode::CmsgQuestGiverCompleteQuest,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_quest_giver_complete_quest
                ),
                define_handler!(
                    Opcode::CmsgQuestGiverChooseReward,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_quest_giver_choose_reward
                ),
                define_handler!(
                    Opcode::CmsgQuestGiverRequestReward,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_quest_giver_request_reward
                ),
                define_handler!(
                    Opcode::CmsgLoot,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_loot
                ),
                define_handler!(
                    Opcode::CmsgLootMoney,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_loot_money
                ),
                define_handler!(
                    Opcode::CmsgLootRelease,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_loot_release
                ),
                define_handler!(
                    Opcode::CmsgItemNameQuery,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_item_name_query
                ),
                define_handler!(
                    Opcode::CmsgAutostoreLootItem,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_autostore_loot_item
                ),
                define_handler!(
                    Opcode::CmsgDestroyItem,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_destroy_item
                ),
                define_handler!(
                    Opcode::CmsgAutoEquipItem,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_auto_equip_item
                ),
                define_handler!(
                    Opcode::CmsgSwapInvItem,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_swap_inv_item
                ),
                define_handler!(
                    Opcode::CmsgSplitItem,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_split_item
                ),
                define_handler!(
                    Opcode::CmsgGameObjectQuery,
                    ProcessImmediately,
                    OpcodeHandler::handle_cmsg_game_object_query
                ),
                define_handler!(
                    Opcode::CmsgUseItem,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_use_item
                ),
                define_handler!(
                    Opcode::MsgMoveTeleportAck,
                    ProcessInMap,
                    OpcodeHandler::handle_msg_move_teleport_ack
                ),
                define_handler!(
                    Opcode::MsgMoveWorldportAck,
                    ProcessInMap,
                    OpcodeHandler::handle_msg_move_worldport_ack
                ),
                define_handler!(
                    Opcode::CmsgBinderActivate,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_binder_activate
                ),
                define_handler!(
                    Opcode::CmsgSetActionButton,
                    ProcessInMap,
                    OpcodeHandler::handle_cmsg_set_action_button
                ),
            ]),
        }
    }

    pub fn get_handler(&self, opcode: u32) -> &(OpcodeProcessingMode, PacketHandler) {
        self.handlers
            .get(&opcode)
            .inspect(|_| {
                trace!("Received {:?} ({:#X})", Opcode::n(opcode).unwrap(), opcode);
            })
            .unwrap_or_else(|| {
                error!(
                    "Received unhandled {:?} ({:#X})",
                    Opcode::n(opcode).unwrap(),
                    opcode
                );
                self.handlers.get(&(Opcode::MsgNullAction as u32)).unwrap()
            })
    }

    pub(crate) fn send_initial_gossip_menu(
        guid: u64,
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
    ) {
        let Some(target_guid) = ObjectGuid::from_raw(guid) else {
            warn!("send_initial_gossip_menu: invalid guid received");
            return;
        };

        let map = session.current_map().unwrap();
        map.world().run(
            |v_player: View<Player>,
             v_creature: View<Creature>,
             v_quest_actor: View<QuestActor>| {
                let my_entity_id = session.player_entity_id().unwrap();
                let player = &v_player[my_entity_id];

                let quest_giver_entity_id = map.lookup_entity_ecs(&target_guid).unwrap();

                let creature = &v_creature[quest_giver_entity_id];

                let creature_template = world_context
                    .data_store
                    .get_creature_template(creature.entry)
                    .expect("unknown creature template id CMSG_QUESTGIVER_HELLO");
                let mut gossip_menu = creature_template
                    .gossip_menu_id
                    .map(|menu_id| {
                        let menu_db_record =
                            world_context.data_store.get_gossip_menu(menu_id).unwrap();

                        GossipMenu::from_db_record(menu_db_record)
                    })
                    .unwrap_or_default();

                if let Ok(quest_actor) = &v_quest_actor.get(quest_giver_entity_id) {
                    for quest_id in quest_actor.quests_started() {
                        let quest_template = world_context
                            .data_store
                            .get_quest_template(*quest_id)
                            .unwrap();
                        match player.quest_status(quest_id).map(|ctx| ctx.status) {
                            None if player.can_start_quest(quest_template) => {
                                gossip_menu.add_quest(*quest_id, QuestGiverStatus::Available)
                            }
                            Some(_) | None => (),
                        }
                    }

                    for quest_id in quest_actor.quests_ended() {
                        match player.quest_status(quest_id).map(|ctx| ctx.status) {
                            Some(PlayerQuestStatus::InProgress) => {
                                gossip_menu.add_quest(*quest_id, QuestGiverStatus::Incomplete);
                            }
                            Some(PlayerQuestStatus::ObjectivesCompleted) => {
                                gossip_menu.add_quest(*quest_id, QuestGiverStatus::Incomplete);
                            }
                            Some(_) | None => (),
                        }
                    }
                }

                let packet = ServerMessage::new(SmsgGossipMessage::from_gossip_menu(
                    &target_guid,
                    &gossip_menu,
                    world_context.data_store.clone(),
                ));

                session.send(&packet).unwrap();
            },
        );
    }
}
