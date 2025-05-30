use binrw::NullString;
use shipyard::{Get, ViewMut};

use crate::ecs::components::melee::Melee;
use crate::ecs::components::unit::Unit;
use crate::entities::object_guid::ObjectGuid;
use crate::entities::player::player_data::ActionButton;
use crate::entities::player::Player;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::repositories::character::CharacterRepository;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};
use crate::session::world_session::{WSRunnableArgs, WorldSessionState};
use crate::shared::response_codes::ResponseCodes;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_char_create(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_char_create: CmsgCharCreate = ClientMessage::read_as(data).unwrap();
        let mut conn = world_context.database.characters.get().unwrap();

        let name_available =
            CharacterRepository::is_name_available(&conn, cmsg_char_create.name.to_string());
        let result = if name_available {
            match Player::create_in_db(
                &mut conn,
                &cmsg_char_create,
                session.account_id,
                world_context.clone(),
            ) {
                Ok(_) => ResponseCodes::CharCreateSuccess,
                Err(_) => ResponseCodes::CharCreateFailed,
            }
        } else {
            ResponseCodes::CharCreateNameInUse
        };

        let packet = ServerMessage::new(SmsgCharCreate {
            result: result as u8,
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_char_enum(
        PacketHandlerArgs {
            session,
            world_context,
            ..
        }: PacketHandlerArgs,
    ) {
        {
            let mut session_state = session.state.write();
            *session_state = WorldSessionState::OnCharactersList;
        }

        let conn = world_context.database.characters.get().unwrap();
        let character_data = CharacterRepository::fetch_characters(
            &conn,
            session.account_id,
            world_context.data_store.clone(),
        );

        let packet = ServerMessage::new(SmsgCharEnum {
            number_of_characters: character_data.len() as u8,
            character_data,
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_char_delete(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_char_delete: CmsgCharDelete = ClientMessage::read_as(data).unwrap();
        let conn = world_context.database.characters.get().unwrap();
        CharacterRepository::delete_character(&conn, cmsg_char_delete, session.account_id);

        let packet = ServerMessage::new(SmsgCharDelete {
            result: ResponseCodes::CharDeleteSuccess as u8,
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_player_login(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_player_login: CmsgPlayerLogin = ClientMessage::read_as(data).unwrap();

        let account_id = session.account_id;
        let conn = world_context.database.characters.get().unwrap();

        let character_data =
            CharacterRepository::fetch_basic_character_data(&conn, cmsg_player_login.guid)
                .expect("Failed to load character from DB");

        assert!(
            character_data.account_id == account_id,
            "Attempt to load a character belonging to another account"
        );

        let msg_set_dungeon_difficulty = ServerMessage::new(MsgSetDungeonDifficulty {
            difficulty: 0, // FIXME
            unk: 1,
            is_in_group: 0, // FIXME after group implementation
        });

        session.send(&msg_set_dungeon_difficulty).unwrap();

        let smsg_login_verify_world = ServerMessage::new(SmsgLoginVerifyWorld {
            map: character_data.position.map_key.map_id,
            position_x: character_data.position.x,
            position_y: character_data.position.y,
            position_z: character_data.position.z,
            orientation: character_data.position.o,
        });

        session.send(&smsg_login_verify_world).unwrap();

        let smsg_account_data_times =
            ServerMessage::new(SmsgAccountDataTimes { data: [0_u32; 32] });

        session.send(&smsg_account_data_times).unwrap();

        let smsg_feature_system_status = ServerMessage::new(SmsgFeatureSystemStatus {
            unk: 2,
            voice_chat_enabled: 0,
        });

        session.send(&smsg_feature_system_status).unwrap();

        let smsg_motd = ServerMessage::new(SmsgMotd {
            line_count: 1,
            message: NullString::from("MOTD"), // TODO: store this in config file
        });

        session.send(&smsg_motd).unwrap();

        session.send_initial_packets_before_add_to_map(&character_data.bindpoint());

        if let Some(map) = world_context
            .map_manager
            .get_map(character_data.position.map_key)
        {
            session.set_map(map.clone());
            session.set_player_guid(ObjectGuid::from_raw(character_data.guid).unwrap());
            map.add_player_on_login(session.clone(), &character_data);

            session.send_initial_packets_after_add_to_map(world_context.clone());
        }
    }

    pub(crate) fn handle_cmsg_logout_request(
        PacketHandlerArgs {
            session,
            world_context,
            ..
        }: PacketHandlerArgs,
    ) {
        let packet = ServerMessage::new(SmsgLogoutResponse {
            reason: 0,
            is_instant_logout: 1,
        });

        session.send(&packet).unwrap();

        let packet = ServerMessage::new(SmsgLogoutComplete {});

        session.send(&packet).unwrap();

        // FIXME: Handle future cases when logout might not be instant
        session.shutdown(
            &mut world_context.database.characters.get().unwrap(),
            world_context.clone(),
        );

        if let Some(map) = session.current_map() {
            let player_guid = &session
                .player_guid()
                .expect("attempt to logout from a session with no player");

            map.remove_player_on_logout(player_guid);
        }
    }

    pub(crate) fn handle_cmsg_name_query(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_name_query: CmsgNameQuery = ClientMessage::read_as(data).unwrap();

        let conn = world_context.database.characters.get().unwrap();

        let char_data =
            CharacterRepository::fetch_basic_character_data(&conn, cmsg_name_query.guid);

        let packet = if let Some(char_data) = char_data {
            ServerMessage::new(SmsgNameQueryResponse {
                guid: char_data.guid,
                name: char_data.name.into(),
                realm_name: 0,
                race: char_data.race as u32,
                class: char_data.class as u32,
                gender: char_data.gender as u32,
                is_name_declined: false,
            })
        } else {
            ServerMessage::new(SmsgNameQueryResponse {
                guid: cmsg_name_query.guid,
                name: "<non-existing character>".into(),
                realm_name: 0,
                race: 0,
                class: 0,
                gender: 0,
                is_name_declined: false,
            })
        };

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_stand_state_change(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg_stand_state_change: CmsgStandStateChange = ClientMessage::read_as(data).unwrap();
        if let Some(map) = session.current_map() {
            if let Some(entity_id) = map.lookup_entity_ecs(&session.player_guid().unwrap()) {
                map.world().run(|mut vm_unit: ViewMut<Unit>| {
                    vm_unit[entity_id].set_stand_state(cmsg_stand_state_change.animstate);
                });
            }
        }

        let packet = ServerMessage::new(SmsgStandStateUpdate {
            animstate: cmsg_stand_state_change.animstate as u8,
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_set_sheathed(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg_set_sheathed: CmsgSetSheathed = ClientMessage::read_as(data).unwrap();
        if let Some(map) = session.current_map() {
            if let Some(entity_id) = map.lookup_entity_ecs(&session.player_guid().unwrap()) {
                map.world().run(|mut vm_melee: ViewMut<Melee>| {
                    vm_melee[entity_id].set_sheath_state(cmsg_set_sheathed.sheath_state);
                });
            }
        }
    }

    pub fn handle_cmsg_set_action_button(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgSetActionButton = ClientMessage::read_as(data).unwrap();

        session.run(&|WSRunnableArgs {
                          map,
                          player_entity_id,
                      }: WSRunnableArgs| {
            map.world().run(|mut vm_player: ViewMut<Player>| {
                if let Ok(mut player) = (&mut vm_player).get(player_entity_id) {
                    if cmsg.data == 0 {
                        player.remove_action_button(cmsg.position as u8);
                        return;
                    }

                    let action_button = ActionButton::from_packed(cmsg.position as u8, cmsg.data);
                    player.add_action_button(action_button);
                }
            });
        });
    }
}
