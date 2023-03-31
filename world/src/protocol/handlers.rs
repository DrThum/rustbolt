use binrw::io::Cursor;
use binrw::{BinReaderExt, NullString};
use futures::future::{BoxFuture, FutureExt};
use lazy_static::lazy_static;
use log::{error, trace};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::datastore::DataStore;
use crate::entities::player::Player;
use crate::entities::update::UpdatableEntity;
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
            Box::new(|data, session: Arc<Mutex<WorldSession>>| $handler(data, session).boxed())
                as PacketHandler,
        )
    };
}

type PacketHandler =
    Box<dyn Send + Sync + Fn(Vec<u8>, Arc<Mutex<WorldSession>>) -> BoxFuture<'static, ()>>;
lazy_static! {
    static ref HANDLERS: HashMap<u32, PacketHandler> = HashMap::from([
        define_handler!(Opcode::MsgNullAction, unhandled),
        define_handler!(Opcode::CmsgCharCreate, handle_cmsg_char_create),
        define_handler!(Opcode::CmsgCharEnum, handle_cmsg_char_enum),
        define_handler!(Opcode::CmsgCharDelete, handle_cmsg_char_delete),
        define_handler!(Opcode::CmsgPlayerLogin, handle_cmsg_player_login),
        define_handler!(Opcode::CmsgPing, handle_cmsg_ping),
        define_handler!(Opcode::CmsgRealmSplit, handle_cmsg_realm_split),
        define_handler!(Opcode::CmsgLogoutRequest, handle_cmsg_logout_request),
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

async fn handle_cmsg_char_create(data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_CHAR_CREATE");
    let session = session.lock().await;

    let mut reader = Cursor::new(data);
    let cmsg_char_create: CmsgCharCreate = reader.read_le().unwrap();
    let mut conn = session.db_pool_char.get().unwrap();

    let name_available =
        CharacterRepository::is_name_available(&conn, cmsg_char_create.name.to_string());
    let data_store = &session.world.data_store;
    let result = if name_available {
        match Player::create(&mut conn, &cmsg_char_create, session.account_id, data_store) {
            Ok(_) => ResponseCodes::CharCreateSuccess,
            Err(_) => ResponseCodes::CharCreateFailed,
        }
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

async fn handle_cmsg_char_enum(_data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_CHAR_ENUM");
    let session = session.lock().await;
    let data_store: &DataStore = &session.world.data_store;

    let conn = session.db_pool_char.get().unwrap();
    let character_data =
        CharacterRepository::fetch_characters(&conn, session.account_id, data_store);

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

async fn handle_cmsg_char_delete(data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_CHAR_DELETE");
    let session = session.lock().await;

    let mut reader = Cursor::new(data);
    let cmsg_char_delete: CmsgCharDelete = reader.read_le().unwrap();

    let conn = session.db_pool_char.get().unwrap();
    CharacterRepository::delete_character(&conn, cmsg_char_delete, session.account_id);

    let packet = ServerMessage::new(SmsgCharDelete {
        result: ResponseCodes::CharDeleteSuccess as u8,
    });

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_CHAR_DELETE");
}

async fn handle_cmsg_player_login(data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_PLAYER_LOGIN");
    let mut session = session.lock().await;

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

    let account_id = session.account_id;
    let data_store: &DataStore = &session.world.data_store;
    let conn = session.db_pool_char.get().unwrap();

    session
        .player
        .load(&conn, account_id, cmsg_player_login.guid, data_store);

    let object_updates: Vec<ObjectUpdate> = session
        .player
        .get_create_data()
        .into_iter()
        .map(|update_data| {
            ObjectUpdate {
                update_type: update_data.update_type as u8,
                packed_guid: update_data.packed_guid,
                object_type: update_data.object_type as u8,
                flags: update_data.flags,
                movement_flags: update_data.movement_flags,
                movement_flags2: 0, // Always 0 in TBC
                timestamp: 0,
                position_x: update_data.position.x,
                position_y: update_data.position.y,
                position_z: update_data.position.z,
                orientation: update_data.position.o,
                fall_time: update_data.fall_time,
                speed_walk: update_data.speed_walk,
                speed_run: update_data.speed_run,
                speed_run_backward: update_data.speed_run_backward,
                speed_swim: update_data.speed_swim,
                speed_swim_backward: update_data.speed_swim_backward,
                speed_flight: update_data.speed_flight,
                speed_flight_backward: update_data.speed_flight_backward,
                speed_turn: update_data.speed_turn,
                unk_highguid: Some(0), // FIXME: Some if flags & UPDATEFLAG_HIGHGUID != 0
                num_mask_blocks: update_data.blocks[0].num_masks,
                mask_blocks: update_data.blocks[0].block_masks.clone(), // FIXME
                data: update_data.blocks[0].data.clone(),               // FIXME
            }
        })
        .collect();

    let smsg_update_object = ServerMessage::new(SmsgUpdateObject {
        // Note: Mangos uses COMPRESSED if data.len() > 100
        updates_count: object_updates.len() as u32,
        has_transport: 0,
        updates: object_updates,
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

async fn unhandled(_data: Vec<u8>, _session: Arc<Mutex<WorldSession>>) {}

async fn handle_cmsg_realm_split(data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_REALM_SPLIT");
    let session = session.lock().await;

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

async fn handle_cmsg_ping(data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_PING");
    let session = session.lock().await;

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

async fn handle_cmsg_update_account_data(data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_UPDATE_ACCOUNT_DATA");
    let session = session.lock().await;

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

async fn handle_cmsg_logout_request(_data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_LOGOUT_REQUEST");
    let session = session.lock().await;

    let packet = ServerMessage::new(SmsgLogoutResponse {
        reason: 0,
        is_instant_logout: 1,
    });

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_LOGOUT_RESPONSE");

    let packet = ServerMessage::new(SmsgLogoutComplete {});

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();
    trace!("Sent SMSG_LOGOUT_COMPLETE");
}
