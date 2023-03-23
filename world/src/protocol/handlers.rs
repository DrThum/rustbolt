use binrw::io::Cursor;
use binrw::{BinReaderExt, NullString};
use futures::future::{BoxFuture, FutureExt};
use lazy_static::lazy_static;
use log::{error, trace};
use std::{collections::HashMap, sync::Arc};

use crate::entities::update::UpdateDataBuilder;
use crate::entities::update_fields::{ObjectFields, UnitFields};
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::repositories::character::CharacterRepository;
use crate::shared::response_codes::ResponseCodes;
use crate::world_session::WorldSession;

use super::opcodes::Opcode;

macro_rules! define_handler {
    ($opcode:expr, $handler:expr) => {
        (
            $opcode as u32,
            Box::new(|data, session: Arc<WorldSession>| $handler(data, session).boxed())
                as PacketHandler,
        )
    };
}

type PacketHandler =
    Box<dyn Send + Sync + Fn(Vec<u8>, Arc<WorldSession>) -> BoxFuture<'static, ()>>;
lazy_static! {
    static ref HANDLERS: HashMap<u32, PacketHandler> = HashMap::from([
        define_handler!(Opcode::MsgNullAction, unhandled),
        define_handler!(Opcode::CmsgCharCreate, handle_cmsg_char_create),
        define_handler!(Opcode::CmsgCharEnum, handle_cmsg_char_enum),
        define_handler!(Opcode::CmsgCharDelete, handle_cmsg_char_delete),
        define_handler!(Opcode::CmsgPlayerLogin, handle_cmsg_player_login),
        define_handler!(Opcode::CmsgPing, handle_cmsg_ping),
        define_handler!(Opcode::CmsgRealmSplit, handle_cmsg_realm_split),
        define_handler!(
            Opcode::CmsgUpdateAccountData,
            handle_cmsg_update_account_data
        ),
    ]);
}

pub fn get_handler(opcode: u32) -> &'static PacketHandler {
    HANDLERS.get(&opcode).unwrap_or_else(|| {
        error!("Received unhandled opcode {:#X}", opcode);
        HANDLERS.get(&(Opcode::MsgNullAction as u32)).unwrap()
    })
}

async fn handle_cmsg_char_create(data: Vec<u8>, session: Arc<WorldSession>) {
    trace!("Received CMSG_CHAR_CREATE");

    let mut reader = Cursor::new(data);
    let cmsg_char_create: CmsgCharCreate = reader.read_le().unwrap();
    let conn = session.db_pool_char.get().unwrap();

    let name_available =
        CharacterRepository::is_name_available(&conn, cmsg_char_create.name.to_string());
    let result = if name_available {
        CharacterRepository::create_character(&conn, cmsg_char_create);
        ResponseCodes::CharCreateSuccess
    } else {
        ResponseCodes::CharCreateNameInUse
    };

    let packet = ServerMessage::new(SmsgCharCreate {
        result: result as u8,
    });

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_CHAR_CREATE");
}

async fn handle_cmsg_char_enum(_data: Vec<u8>, session: Arc<WorldSession>) {
    trace!("Received CMSG_CHAR_ENUM");

    let conn = session.db_pool_char.get().unwrap();
    let character_data = CharacterRepository::fetch_characters(conn, session.account_id);

    let packet = ServerMessage::new(SmsgCharEnum {
        number_of_characters: character_data.len() as u8,
        character_data,
    });

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_CHAR_ENUM");
}

async fn handle_cmsg_char_delete(data: Vec<u8>, session: Arc<WorldSession>) {
    trace!("Received CMSG_CHAR_DELETE");
    let mut reader = Cursor::new(data);
    let cmsg_char_delete: CmsgCharDelete = reader.read_le().unwrap();

    let conn = session.db_pool_char.get().unwrap();
    CharacterRepository::delete_character(conn, cmsg_char_delete, session.account_id);

    let packet = ServerMessage::new(SmsgCharDelete {
        result: 0x3B, // TODO: Enum - CHAR_DELETE_SUCCESS
    });

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_CHAR_DELETE");
}

async fn handle_cmsg_player_login(data: Vec<u8>, session: Arc<WorldSession>) {
    trace!("Received CMSG_PLAYER_LOGIN");
    let mut reader = Cursor::new(data);
    let cmsg_player_login: CmsgPlayerLogin = reader.read_le().unwrap();

    let msg_set_dungeon_difficulty = ServerMessage::new(MsgSetDungeonDifficulty {
        difficulty: 0, // FIXME
        unk: 1,
        is_in_group: 0, // FIXME after group implementation
    });

    msg_set_dungeon_difficulty
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent MSG_SET_DUNGEON_DIFFICULTY");

    let smsg_login_verify_world = ServerMessage::new(SmsgLoginVerifyWorld {
        map: 0,
        position_x: -8953.95,
        position_y: 521.019,
        position_z: 96.5399,
        orientation: 3.83972,
    });

    smsg_login_verify_world
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_LOGIN_VERIFY_WORLD");

    let smsg_account_data_times = ServerMessage::new(SmsgAccountDataTimes { data: [0_u32; 32] });

    smsg_account_data_times
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_ACCOUNT_DATA_TIMES");

    let smsg_feature_system_status = ServerMessage::new(SmsgFeatureSystemStatus {
        unk: 2,
        voice_chat_enabled: 0,
    });

    smsg_feature_system_status
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_FEATURE_SYSTEM_STATUS");

    let smsg_motd = ServerMessage::new(SmsgMotd {
        line_count: 1,
        message: NullString::from("MOTD"), // TODO: store this in config file
    });

    smsg_motd
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_MOTD");

    let smsg_set_rest_start = ServerMessage::new(SmsgSetRestStart { rest_start: 0 });

    smsg_set_rest_start
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_SET_REST_START");

    let smsg_bindpointupdate = ServerMessage::new(SmsgBindpointupdate {
        homebind_x: -8953.95,
        homebind_y: 521.019,
        homebind_z: 96.5399,
        homebind_map_id: 0,
        homebind_area_id: 85,
    });

    smsg_bindpointupdate
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_BINDPOINTUPDATE");

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

    smsg_tutorial_flags
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_TUTORIAL_FLAGS");

    let smsg_login_settimespeed = ServerMessage::new(SmsgLoginSettimespeed {
        timestamp: 0, // Maybe not zero?
        game_speed: 0.01666667,
    });

    smsg_login_settimespeed
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_LOGIN_SETTIMESPEED");

    let mut conn = session.db_pool_char.get().unwrap();
    let character = CharacterRepository::fetch_basic_character_data(
        &mut conn,
        cmsg_player_login.guid,
        session.account_id,
    )
    .unwrap();

    let mut update_data_builder = UpdateDataBuilder::new();
    update_data_builder.add_u64(ObjectFields::ObjectFieldGuid.into(), cmsg_player_login.guid);
    update_data_builder.add_u32(ObjectFields::ObjectFieldType.into(), 25);
    update_data_builder.add_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0);
    update_data_builder.add_u32(UnitFields::UnitFieldHealth.into(), 100);
    update_data_builder.add_u32(UnitFields::UnitFieldMaxhealth.into(), 100);
    update_data_builder.add_u32(UnitFields::UnitFieldLevel.into(), 70);
    update_data_builder.add_u32(UnitFields::UnitFieldFactiontemplate.into(), 469);
    update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 0, character.0); // race
    update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 1, character.1); // class
    update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 2, character.2); // gender
    update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 3, 0); // powertype, 0 = MANA
    update_data_builder.add_u32(UnitFields::UnitFieldDisplayid.into(), 1478);
    update_data_builder.add_u32(UnitFields::UnitFieldNativedisplayid.into(), 1478);
    let update_data = update_data_builder.build();

    let smsg_update_object = ServerMessage::new(SmsgUpdateObject {
        // Note: Mangos uses COMPRESSED if data.len() > 100
        block_count: 1,
        has_transport: 0,
        update_type: 3,
        packed_guid_mask: 1, // TODO: Properly implement packed guids
        packed_guid_guid: cmsg_player_login.guid as u8,
        object_type: 4,
        flags: 0x71, // PDATEFLAG_HIGHGUID | UPDATEFLAG_LIVING |
        // UPDATEFLAG_STATIONARY_POSITION = 0x10 | 0x20 | 0x40 = 0x70 |
        // UPDATEFLAG_SELF = 0x1 = 0x71
        movement_flags: 0,
        movement_flags2: 0,
        timestamp: 0,
        position_x: -8953.95,
        position_y: 521.019,
        position_z: 96.5399,
        orientation: 3.83972,
        fall_time: 0,
        speed_walk: 1.0,
        speed_run: 70.0,
        speed_run_backward: 4.5,
        speed_swim: 0.0,
        speed_swim_backward: 0.0,
        speed_flight: 70.0,
        speed_flight_backward: 4.5,
        speed_turn: 3.1415,
        unk_highguid: 0,
        num_mask_blocks: update_data.num_masks,
        mask_blocks: update_data.block_masks,
        data: update_data.data,
    });

    smsg_update_object
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent initial SMSG_UPDATE_OBJECT for player");

    let smsg_init_world_states = ServerMessage::new(SmsgInitWorldStates {
        map_id: 0,
        zone_id: 85,
        area_id: 154, // Deathknell
        block_count: 0,
    });

    smsg_init_world_states
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_INIT_WORLD_STATES");

    let smsg_time_sync_req = ServerMessage::new(SmsgTimeSyncReq { sync_counter: 0 });

    smsg_time_sync_req
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_TIME_SYNC_REQ");
}

async fn unhandled(_data: Vec<u8>, _session: Arc<WorldSession>) {}

async fn handle_cmsg_realm_split(data: Vec<u8>, session: Arc<WorldSession>) {
    trace!("Received CMSG_REALM_SPLIT");
    let mut reader = Cursor::new(data);
    let cmsg_realm_split: CmsgRealmSplit = reader.read_le().unwrap();

    let packet = ServerMessage::new(SmsgRealmSplit {
        client_state: cmsg_realm_split.client_state,
        realm_state: 0x00,
        split_date: binrw::NullString::from("01/01/01"),
    });

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_REALM_SPLIT");
}

async fn handle_cmsg_ping(data: Vec<u8>, session: Arc<WorldSession>) {
    trace!("Received CMSG_PING");
    let mut reader = Cursor::new(data);
    let cmsg_ping: CmsgPing = reader.read_le().unwrap();

    let packet = ServerMessage::new(SmsgPong {
        ping: cmsg_ping.ping,
    });

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_PONG");
}

async fn handle_cmsg_update_account_data(data: Vec<u8>, session: Arc<WorldSession>) {
    trace!("Received CMSG_UPDATE_ACCOUNT_DATA");
    let mut reader = Cursor::new(data);
    let cmsg_update_account_data: CmsgUpdateAccountData = reader.read_le().unwrap();

    let packet = ServerMessage::new(SmsgUpdateAccountData {
        account_data_id: cmsg_update_account_data.account_data_id,
        data: 0,
    });

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_UPDATE_ACCOUNT_DATA_ID");
}
