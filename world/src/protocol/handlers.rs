use binrw::io::Cursor;
use binrw::BinReaderExt;
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
    CharEnumData, CharEnumEquip, CmsgCharCreate, CmsgCharDelete, CmsgPing, CmsgRealmSplit,
    SmsgCharCreate, SmsgCharDelete, SmsgCharEnum, SmsgPong, SmsgRealmSplit,
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
        define_handler!(Opcode::CmsgPing, handle_cmsg_ping),
        define_handler!(Opcode::CmsgRealmSplit, handle_cmsg_realm_split),
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
