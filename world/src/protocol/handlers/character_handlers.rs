use binrw::NullString;
use chrono::{Datelike, Timelike};
use std::sync::Arc;

use crate::entities::player::Player;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::repositories::character::CharacterRepository;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::{WorldSession, WorldSessionState};
use crate::shared::response_codes::ResponseCodes;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_char_create(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_char_create: CmsgCharCreate = ClientMessage::read_as(data).unwrap();
        let mut conn = world_context.database.characters.get().unwrap();

        let name_available =
            CharacterRepository::is_name_available(&conn, cmsg_char_create.name.to_string());
        let result = if name_available {
            match Player::create(
                &mut conn,
                &cmsg_char_create,
                session.account_id,
                world_context.data_store.clone(),
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
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        _data: Vec<u8>,
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
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
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
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_player_login: CmsgPlayerLogin = ClientMessage::read_as(data).unwrap();

        let account_id = session.account_id;
        let conn = world_context.database.characters.get().unwrap();

        {
            let mut player = session.player.write();
            player.load(
                &conn,
                account_id,
                cmsg_player_login.guid,
                world_context.clone(),
            );
        }

        let msg_set_dungeon_difficulty = ServerMessage::new(MsgSetDungeonDifficulty {
            difficulty: 0, // FIXME
            unk: 1,
            is_in_group: 0, // FIXME after group implementation
        });

        session.send(&msg_set_dungeon_difficulty).unwrap();

        {
            let player_position = session.player.read().position().clone();
            let smsg_login_verify_world = ServerMessage::new(SmsgLoginVerifyWorld {
                map: player_position.map_key.map_id,
                position_x: player_position.x,
                position_y: player_position.y,
                position_z: player_position.z,
                orientation: player_position.o,
            });

            session.send(&smsg_login_verify_world).unwrap();
        }

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

        let smsg_set_rest_start = ServerMessage::new(SmsgSetRestStart { rest_start: 0 });

        session.send(&smsg_set_rest_start).unwrap();

        // TODO
        let smsg_bindpointupdate = ServerMessage::new(SmsgBindpointupdate {
            homebind_x: -8953.95,
            homebind_y: 521.019,
            homebind_z: 96.5399,
            homebind_map_id: 0,
            homebind_area_id: 85,
        });

        session.send(&smsg_bindpointupdate).unwrap();

        let smsg_tutorial_flags = ServerMessage::new(SmsgTutorialFlags {
            tutorial_data0: 0, // FIXME: 0xFFFFFFFF to disable tutorials
            tutorial_data1: 0,
            tutorial_data2: 0,
            tutorial_data3: 0,
            tutorial_data4: 0,
            tutorial_data5: 0,
            tutorial_data6: 0,
            tutorial_data7: 0,
        });

        session.send(&smsg_tutorial_flags).unwrap();

        // The client expects a specific format which is not unix timestamp
        // See secsToTimeBitFields in MaNGOS
        let timestamp: u32 = {
            let now = chrono::Local::now();

            let year = now.year() as u32;
            let month = now.month();
            let month_day = now.day() - 1;
            let weekday = now.weekday().number_from_sunday();
            let hour = now.hour();
            let minutes = now.minute();

            (year << 24)
                | (month << 20)
                | (month_day << 14)
                | (weekday << 11)
                | (hour << 6)
                | minutes
        };

        let smsg_login_settimespeed = ServerMessage::new(SmsgLoginSettimespeed {
            timestamp,
            game_speed: 0.01666667,
        });

        session.send(&smsg_login_settimespeed).unwrap();

        {
            world_context.map_manager.add_session_to_map(
                session.clone(),
                world_context.clone(),
                session.player.clone(),
            );

            let mut session_state = session.state.write();
            *session_state = WorldSessionState::InWorld;
        }

        // FIXME: hardcoded position
        let smsg_init_world_states = ServerMessage::new(SmsgInitWorldStates {
            map_id: 0,
            zone_id: 85,
            area_id: 154, // Deathknell
            block_count: 0,
        });

        session.send(&smsg_init_world_states).unwrap();

        WorldSession::reset_time_sync(session, world_context);
    }

    pub(crate) fn handle_cmsg_logout_request(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        _data: Vec<u8>,
    ) {
        let packet = ServerMessage::new(SmsgLogoutResponse {
            reason: 0,
            is_instant_logout: 1,
        });

        session.send(&packet).unwrap();

        let packet = ServerMessage::new(SmsgLogoutComplete {});

        session.send(&packet).unwrap();

        {
            let player_guid = session.player.read().guid().clone();
            let current_map_key = session.player.read().current_map();
            world_context
                .map_manager
                .remove_player_from_map(&player_guid, current_map_key);
        }

        // FIXME: Handle future cases when logout might not be instant
        session.cleanup_on_world_leave(&mut world_context.database.characters.get().unwrap());
    }

    pub(crate) fn handle_cmsg_name_query(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
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
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_stand_state_change: CmsgStandStateChange = ClientMessage::read_as(data).unwrap();
        {
            session
                .player
                .write()
                .set_stand_state(cmsg_stand_state_change.animstate);
        }

        let packet = ServerMessage::new(SmsgStandStateUpdate {
            animstate: cmsg_stand_state_change.animstate as u8,
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_set_sheathed(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_set_sheathed: CmsgSetSheathed = ClientMessage::read_as(data).unwrap();
        session
            .player
            .write()
            .set_sheath_state(cmsg_set_sheathed.sheath_state);
    }
}
