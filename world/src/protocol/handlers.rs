use binrw::io::Cursor;
use binrw::{BinReaderExt, NullString};
use futures::future::{BoxFuture, FutureExt};
use lazy_static::lazy_static;
use log::{error, trace};
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::named_params;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use wow_srp::tbc_header::HeaderCrypto;

use crate::constants::InventoryType;
use crate::protocol::packets::{
    CharEnumData, CharEnumEquip, CmsgCharCreate, CmsgCharDelete, CmsgPing, CmsgPlayerLogin,
    CmsgRealmSplit, CmsgUpdateAccountData, SmsgAccountDataTimes, SmsgBindpointupdate,
    SmsgCharCreate, SmsgCharDelete, SmsgCharEnum, SmsgFeatureSystemStatus, SmsgLoginSettimespeed,
    SmsgLoginVerifyWorld, SmsgMotd, SmsgPong, SmsgRealmSplit, SmsgSetRestStart, SmsgTimeSyncReq,
    SmsgTutorialFlags, SmsgUpdateAccountData, SmsgUpdateObject,
};
use crate::protocol::server::ServerMessage;
use crate::world_session::WorldSession;

use super::opcodes::Opcode;

macro_rules! define_handler {
    ($opcode:expr, $handler:expr) => {
        (
            $opcode as u32,
            Box::new(
                |data, crypto: Arc<Mutex<HeaderCrypto>>, session: Arc<Mutex<WorldSession>>| {
                    $handler(data, crypto, session).boxed()
                },
            ) as PacketHandler,
        )
    };
}

type PacketHandler = Box<
    dyn Send
        + Sync
        + Fn(Vec<u8>, Arc<Mutex<HeaderCrypto>>, Arc<Mutex<WorldSession>>) -> BoxFuture<'static, ()>,
>;
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

async fn handle_cmsg_char_create(
    data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    session: Arc<Mutex<WorldSession>>,
) {
    fn create_char(conn: PooledConnection<SqliteConnectionManager>, source: CmsgCharCreate) {
        // let mut stmt_check_name = conn.prepare_cached("SELECT COUNT(*) FROM characters WHERE name = ?").unwrap();
        // // TODO

        let mut stmt_create = conn.prepare_cached("INSERT INTO characters (guid, account_id, name, race, class, gender, skin, face, hairstyle, haircolor, facialstyle) VALUES (NULL, :account_id, :name, :race, :class, :gender, :skin, :face, :hairstyle, :haircolor, :facialstyle)").unwrap();
        stmt_create
            .execute(named_params! {
                ":account_id": 1, /* FIXME: Add account id to WorldSession */
                ":name": source.name.to_string(),
                ":race": source.race,
                ":class": source.class,
                ":gender": source.gender,
                ":skin": source.skin,
                ":face": source.face,
                ":hairstyle": source.hairstyle,
                ":haircolor": source.haircolor,
                ":facialstyle": source.facialstyle,
            })
            .unwrap();
    }

    trace!("Received CMSG_CHAR_CREATE");

    let mut reader = Cursor::new(data);
    let cmsg_char_create: CmsgCharCreate = reader.read_le().unwrap();
    let session_guard = session.lock().await;
    let conn = session_guard.db_pool_char.get().unwrap();
    create_char(conn, cmsg_char_create);

    let packet = ServerMessage::new(SmsgCharCreate {
        result: 0x2F, // TODO: Enum
    });

    let socket = Arc::clone(&session_guard.socket);
    packet.send(socket, encryption).await.unwrap();
    trace!("Sent SMSG_CHAR_CREATE");
}

async fn handle_cmsg_char_enum(
    _data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    session: Arc<Mutex<WorldSession>>,
) {
    fn fetch_chars(conn: PooledConnection<SqliteConnectionManager>) -> Vec<CharEnumData> {
        let mut stmt = conn.prepare_cached("SELECT guid, name, race, class, gender, skin, face, hairstyle, haircolor, facialstyle FROM characters WHERE account_id = 1").unwrap(); // FIXME: Account id
        let chars = stmt
            .query_map([], |row| {
                let equipment = vec![
                    InventoryType::Head,
                    InventoryType::Neck,
                    InventoryType::Shoulders,
                    InventoryType::Body,
                    InventoryType::Chest,
                    InventoryType::Waist,
                    InventoryType::Legs,
                    InventoryType::Feet,
                    InventoryType::Wrists,
                    InventoryType::Hands,
                    InventoryType::Finger,
                    InventoryType::Finger,
                    InventoryType::Trinket,
                    InventoryType::Trinket,
                    InventoryType::Cloak,
                    InventoryType::WeaponMainHand,
                    InventoryType::WeaponOffHand,
                    InventoryType::Ranged,
                    InventoryType::Tabard,
                    InventoryType::NonEquip,
                ]
                .into_iter()
                .map(|inv_type| CharEnumEquip::none(inv_type))
                .collect();

                Ok(CharEnumData {
                    guid: row.get("guid").unwrap(),
                    name: row.get::<&str, String>("name").unwrap().try_into().unwrap(),
                    race: row.get("race").unwrap(),
                    class: row.get("class").unwrap(),
                    gender: row.get("gender").unwrap(),
                    skin: row.get("skin").unwrap(),
                    face: row.get("face").unwrap(),
                    hairstyle: row.get("hairstyle").unwrap(),
                    haircolor: row.get("haircolor").unwrap(),
                    facialstyle: row.get("facialstyle").unwrap(),
                    level: 70,
                    area: 85,
                    map: 0,
                    position_x: 0.0,
                    position_y: 0.0,
                    position_z: 0.0,
                    guild_id: 0,
                    flags: 0,
                    first_login: 1, // FIXME: bool
                    pet_display_id: 0,
                    pet_level: 0,
                    pet_family: 0,
                    equipment,
                })
            })
            .unwrap();

        chars
            .filter(|res| res.is_ok())
            .map(|res| res.unwrap())
            .into_iter()
            .collect()
    }

    trace!("Received CMSG_CHAR_ENUM");

    let session_guard = session.lock().await;
    let conn = session_guard.db_pool_char.get().unwrap();
    let character_data = fetch_chars(conn);

    let packet = ServerMessage::new(SmsgCharEnum {
        number_of_characters: character_data.len() as u8,
        character_data,
    });

    let socket = Arc::clone(&session_guard.socket);
    packet.send(socket, encryption).await.unwrap();
    trace!("Sent SMSG_CHAR_ENUM");
}

async fn handle_cmsg_char_delete(
    data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    session: Arc<Mutex<WorldSession>>,
) {
    fn delete_char(conn: PooledConnection<SqliteConnectionManager>, source: CmsgCharDelete) {
        let mut stmt_delete = conn
            .prepare_cached(
                "DELETE FROM characters WHERE guid = :guid AND account_id = :account_id",
            )
            .unwrap();
        stmt_delete
            .execute(named_params! {
                ":guid": source.guid,
                ":account_id": 1, /* FIXME: Add account id to WorldSession */
            })
            .unwrap();
    }

    trace!("Received CMSG_CHAR_DELETE");
    let mut reader = Cursor::new(data);
    let cmsg_char_delete: CmsgCharDelete = reader.read_le().unwrap();

    let session_guard = session.lock().await;
    let conn = session_guard.db_pool_char.get().unwrap();
    delete_char(conn, cmsg_char_delete);

    let packet = ServerMessage::new(SmsgCharDelete {
        result: 0x3B, // TODO: Enum - CHAR_DELETE_SUCCESS
    });

    let socket = Arc::clone(&session_guard.socket);
    packet.send(socket, encryption).await.unwrap();
    trace!("Sent SMSG_CHAR_DELETE");
}

async fn handle_cmsg_player_login(
    data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    session: Arc<Mutex<WorldSession>>,
) {
    fn fetch_basic_char_data(
        conn: &mut PooledConnection<SqliteConnectionManager>,
        guid: u64,
    ) -> Option<(u8, u8, u8, u32, u32, u32, u32)> {
        let mut stmt = conn
            .prepare("SELECT race, class, gender, haircolor, hairstyle, face, skin FROM characters WHERE guid = :guid AND account_id = :account_id")
            .unwrap();
        let mut rows = stmt
            .query(named_params! {
                ":guid": guid,
                ":account_id": 1
            })
            .unwrap(); // FIXME: Add account id to WorldSession

        if let Some(row) = rows.next().unwrap() {
            Some((
                row.get("race").unwrap(),
                row.get("class").unwrap(),
                row.get("gender").unwrap(),
                row.get("haircolor").unwrap(),
                row.get("hairstyle").unwrap(),
                row.get("face").unwrap(),
                row.get("gender").unwrap(),
            ))
        } else {
            None
        }
    }

    trace!("Received CMSG_PLAYER_LOGIN");
    let mut reader = Cursor::new(data);
    let cmsg_player_login: CmsgPlayerLogin = reader.read_le().unwrap();

    let smsg_login_verify_world = ServerMessage::new(SmsgLoginVerifyWorld {
        map: 0,
        position_x: -8953.95,
        position_y: 521.019,
        position_z: 96.5399,
        orientation: 3.83972,
    });

    let session_guard = session.lock().await;
    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_login_verify_world
        .send(socket, encryption_copy)
        .await
        .unwrap();
    trace!("Sent SMSG_LOGIN_VERIFY_WORLD");

    let smsg_tutorial_flags = ServerMessage::new(SmsgTutorialFlags {
        tutorial_data0: 0, // FIXME: 0xFF to disable tutorials
        tutorial_data1: 0,
        tutorial_data2: 0,
        tutorial_data3: 0,
        tutorial_data4: 0,
        tutorial_data5: 0,
        tutorial_data6: 0,
        tutorial_data7: 0,
    });

    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_tutorial_flags
        .send(socket, encryption_copy)
        .await
        .unwrap();
    trace!("Sent SMSG_TUTORIAL_FLAGS");

    let smsg_account_data_times = ServerMessage::new(SmsgAccountDataTimes { data: [0_u8; 32] });

    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_account_data_times
        .send(socket, encryption_copy)
        .await
        .unwrap();
    trace!("Sent SMSG_ACCOUNT_DATA_TIMES");

    let smsg_feature_system_status = ServerMessage::new(SmsgFeatureSystemStatus {
        unk: 2,
        voice_chat_enabled: 0,
    });

    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_feature_system_status
        .send(socket, encryption_copy)
        .await
        .unwrap();
    trace!("Sent SMSG_FEATURE_SYSTEM_STATUS");

    let smsg_motd = ServerMessage::new(SmsgMotd {
        line_count: 1,
        message: NullString::from("MOTD"),
    });

    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_motd.send(socket, encryption_copy).await.unwrap();
    trace!("Sent SMSG_MOTD");

    let smsg_login_settimespeed = ServerMessage::new(SmsgLoginSettimespeed {
        timestamp: 0,
        game_speed: 0.01666667,
    });

    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_login_settimespeed
        .send(socket, encryption_copy)
        .await
        .unwrap();
    trace!("Sent SMSG_LOGIN_SETTIMESPEED");

    // https://github.com/AscEmu/AscEmu/blob/b4740618504b4ef90cc117eafbf1b832df90ed3b/src/world/Objects/Object.cpp#L401
    // https://github.com/azerothcore/SunstriderCore/blob/master/src/server/game/Entities/Object/Object.cpp

    // TODO: SMSG_TIME_SYNC_REQ or the player can't move
    // SendInitialPacketsBeforeAddToMap: https://github.com/superwow/TrinityCoreOne/blob/4b111d5fb1664b51431df0acdafca973fe5c2b8f/src/server/game/Entities/Player/Player.cpp#L22069
    // same with AfterAddToMap just after
    // https://www.getmangos.eu/forums/topic/10626-need-help-with-player-login/
    //
    //
    // UpdateData::BuildPacket + Object::BuildCreateUpdateBlockForPlayer
    // Map.cpp:2349
    //
    // BuildValuesUpdateBlockForPlayer used? -> No

    // Check Player::Create

    let mut conn = session_guard.db_pool_char.get().unwrap();
    let character = fetch_basic_char_data(&mut conn, cmsg_player_login.guid).unwrap();
    let smsg_update_object = ServerMessage::new(SmsgUpdateObject {
        // Note: Mangos uses COMPRESSED
        // if data.len() > 100
        block_count: 1,                                 // OK
        has_transport: 0,                               // OK
        update_type: 3,                                 // OK
        packed_guid_mask: 1,                            // should be OK but check how to pack a GUID
        packed_guid_guid: cmsg_player_login.guid as u8, // same ^
        object_type: 4,                                 // OK - TYPEID_PLAYER
        flags: 0x71, // Seems OK - UPDATEFLAG_HIGHGUID | UPDATEFLAG_LIVING |
        // UPDATEFLAG_STATIONARY_POSITION = 0x10 | 0x20 | 0x40 = 0x70 |
        // UPDATEFLAG_SELF = 0x1 = 0x71
        movement_flags: 0,  // Not sure
        movement_flags2: 0, // Pretty sure
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
        num_mask_blocks: 50, // 50
        mask_blocks: vec![],
        data: vec![],
    });
    /*     object_field_guid: cmsg_player_login.guid,
    object_field_type: 25,
    unit_field_scale: 1.0,
    object_health: 100,
    object_power: 100,
    object_max_health: 100,
    object_max_power: 100,
    level: 70,
    faction_template: 469,
    race: character.0,
    class: character.1,
    gender: character.2,
    power: 0,
    bounding_radius: 1.0,
    combat_reach: 1.5,
    display_id: 1478,
    native_display_id: 1478,
    base_attack_time_mainhand: 2000.0,
    base_attack_time_offhand: 2000.0,
    ranged_attack_time: 2000.0,
    unit_mod_cast_speed: 1.0,
    unit_field_base_mana: 100,
    unit_field_base_health: 100,
    unit_field_bytes_2: 0x2800,
    player_bytes: (character.3 << 24) | (character.4 << 16) | (character.5 << 8) | character.6,
    player_bytes_2: 0x02000006, // facial hair TODO
    player_bytes_3: character.2 as u32,
    player_xp: 0,
    player_field_max_level: 70,*/

    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_update_object
        .send(socket, encryption_copy)
        .await
        .unwrap();
    trace!("Sent initial SMSG_UPDATE_OBJECT for player");

    let smsg_time_sync_req = ServerMessage::new(SmsgTimeSyncReq { sync_counter: 1 });

    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_time_sync_req
        .send(socket, encryption_copy)
        .await
        .unwrap();
    trace!("Sent SMSG_TIME_SYNC_REQ");

    let smsg_set_rest_start = ServerMessage::new(SmsgSetRestStart { rest_start: 0 });

    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_set_rest_start
        .send(socket, encryption_copy)
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

    let socket = Arc::clone(&session_guard.socket);
    let encryption_copy = Arc::clone(&encryption);
    smsg_bindpointupdate
        .send(socket, encryption_copy)
        .await
        .unwrap();
    trace!("Sent SMSG_BINDPOINTUPDATE");
}

async fn unhandled(
    _data: Vec<u8>,
    _encryption: Arc<Mutex<HeaderCrypto>>,
    _session: Arc<Mutex<WorldSession>>,
) {
}

async fn handle_cmsg_realm_split(
    data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    session: Arc<Mutex<WorldSession>>,
) {
    trace!("Received CMSG_REALM_SPLIT");
    let mut reader = Cursor::new(data);
    let cmsg_realm_split: CmsgRealmSplit = reader.read_le().unwrap();

    let packet = ServerMessage::new(SmsgRealmSplit {
        client_state: cmsg_realm_split.client_state,
        realm_state: 0x00,
        split_date: binrw::NullString::from("01/01/01"),
    });

    let socket = Arc::clone(&session.lock().await.socket);
    packet.send(socket, encryption).await.unwrap();
    trace!("Sent SMSG_REALM_SPLIT");
}

async fn handle_cmsg_ping(
    data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    session: Arc<Mutex<WorldSession>>,
) {
    trace!("Received CMSG_PING");
    let mut reader = Cursor::new(data);
    let cmsg_ping: CmsgPing = reader.read_le().unwrap();

    let packet = ServerMessage::new(SmsgPong {
        ping: cmsg_ping.ping,
    });

    let socket = Arc::clone(&session.lock().await.socket);
    packet.send(socket, encryption).await.unwrap();
    trace!("Sent SMSG_PONG");
}

async fn handle_cmsg_update_account_data(
    data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    session: Arc<Mutex<WorldSession>>,
) {
    trace!("Received CMSG_UPDATE_ACCOUNT_DATA");
    let mut reader = Cursor::new(data);
    let cmsg_update_account_data: CmsgUpdateAccountData = reader.read_le().unwrap();

    let packet = ServerMessage::new(SmsgUpdateAccountData {
        account_data_id: cmsg_update_account_data.account_data_id,
        data: 0,
    });

    let socket = Arc::clone(&session.lock().await.socket);
    packet.send(socket, encryption).await.unwrap();
    trace!("Sent SMSG_UPDATE_ACCOUNT_DATA_ID");
}
