use binrw::io::Cursor;
use binrw::{BinReaderExt, NullString};
use futures::future::{BoxFuture, FutureExt};
use lazy_static::lazy_static;
use log::{error, trace};
use std::time::{SystemTime, UNIX_EPOCH};
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
        define_handler!(Opcode::CmsgItemQuerySingle, handle_cmsg_item_query_single),
        define_handler!(Opcode::CmsgNameQuery, handle_cmsg_name_query),
        define_handler!(Opcode::CmsgQueryTime, handle_cmsg_query_time),
        define_handler!(
            Opcode::CmsgUpdateAccountData,
            handle_cmsg_update_account_data
        ),
    ]);
}

pub fn get_handler(opcode: u32) -> &'static PacketHandler {
    HANDLERS.get(&opcode).unwrap_or_else(|| {
        error!(
            "Received unhandled opcode {:?} ({:#X})",
            Opcode::n(opcode).unwrap(),
            opcode
        );
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

    let account_id = session.account_id;
    let world = *(session.world);
    let conn = session.db_pool_char.get().unwrap();

    session
        .player
        .load(&conn, account_id, cmsg_player_login.guid, world);

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

    let player_position = session.player.position();
    let smsg_login_verify_world = ServerMessage::new(SmsgLoginVerifyWorld {
        map: player_position.map,
        position_x: player_position.x,
        position_y: player_position.y,
        position_z: player_position.z,
        orientation: player_position.o,
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

    // TODO
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

    let update_data = session
        .player
        .get_create_data(session.player.guid().raw(), *session.world);
    let smsg_update_object = ServerMessage::new(SmsgUpdateObject {
        updates_count: update_data.len() as u32,
        has_transport: false,
        updates: update_data,
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
    let mut session = session.lock().await;

    let mut reader = Cursor::new(data);
    let cmsg_ping: CmsgPing = reader.read_le().unwrap();

    session.client_latency = cmsg_ping.latency;

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

async fn handle_cmsg_item_query_single(data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_ITEM_QUERY_SINGLE");
    let session = session.lock().await;

    let mut reader = Cursor::new(data);
    let cmsg_item_query_single: CmsgItemQuerySingle = reader.read_le().unwrap();

    let data_store = &session.world.data_store;

    let packet = if let Some(item) = data_store.get_item_template(cmsg_item_query_single.item_id) {
        ServerMessage::new(SmsgItemQuerySingleResponse {
            result: None,
            template: Some(ItemQueryResponse {
                item_id: item.entry,
                item_class: item.class,
                item_subclass: item.subclass,
                item_unk: -1,
                name: item.name.clone().into(),
                name2: 0,
                name3: 0,
                name4: 0,
                display_id: item.display_id,
                quality: item.quality,
                flags: item.flags,
                buy_price: item.buy_price,
                sell_price: item.sell_price,
                inventory_type: item.inventory_type,
                allowable_class: item.allowable_class,
                allowable_race: item.allowable_race,
                item_level: item.item_level,
                required_level: item.required_level,
                required_skill: item.required_skill,
                required_skill_rank: item.required_skill,
                required_spell: item.required_spell,
                required_honor_rank: item.required_honor_rank,
                required_city_rank: item.required_city_rank,
                required_reputation_faction: item.required_reputation_faction,
                required_reputation_rank: item.required_reputation_rank,
                max_count: item.max_count,
                max_stack_count: item.max_stack_count,
                container_slots: item.container_slots,
                stats: &item.stats,
                damages: &item.damages,
                armor: item.armor,
                resist_holy: item.holy_res,
                resist_fire: item.fire_res,
                resist_nature: item.nature_res,
                resist_frost: item.frost_res,
                resist_shadow: item.shadow_res,
                resist_arcane: item.arcane_res,
                delay: item.delay,
                ammo_type: item.ammo_type,
                ranged_mod_range: item.ranged_mod_range,
                spells: &item.spells,
                bonding: item.bonding,
                description: item.description.clone().into(),
                page_text: item.page_text,
                language_id: item.language_id,
                page_material: item.page_material,
                start_quest: item.start_quest,
                lock_id: item.lock_id,
                material: item.material,
                sheath: item.sheath,
                random_property: item.random_property,
                random_suffix: item.random_suffix,
                block: item.block,
                item_set: item.itemset,
                max_durability: item.max_durability,
                area: item.area,
                map: item.map,
                bag_family: item.bag_family,
                totem_category: item.totem_category,
                sockets: &item.sockets,
                socket_bonus: item.socket_bonus,
                gem_properties: item.gem_properties,
                required_enchantment_skill: item.required_disenchant_skill as i32,
                armor_damage_modifier: item.armor_damage_modifier,
                duration: item.duration,
            }),
        })
    } else {
        ServerMessage::new(SmsgItemQuerySingleResponse {
            result: Some(cmsg_item_query_single.item_id | 0x80000000),
            template: None,
        })
    };

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();

    trace!("Sent SMSG_ITEM_QUERY_SINGLE_RESPONSE");
}

async fn handle_cmsg_name_query(data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_NAME_QUERY");

    let mut reader = Cursor::new(data);
    let cmsg_name_query: CmsgNameQuery = reader.read_le().unwrap();

    let session = session.lock().await;
    let conn = session.db_pool_char.get().unwrap();

    let char_data = CharacterRepository::fetch_basic_character_data(&conn, cmsg_name_query.guid);

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

    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();

    trace!("Sent SMSG_NAME_QUERY_RESPONSE");
}

async fn handle_cmsg_query_time(_data: Vec<u8>, session: Arc<Mutex<WorldSession>>) {
    trace!("Received CMSG_QUERY_TIME");

    let now = SystemTime::now();
    let seconds_since_epoch = now
        .duration_since(UNIX_EPOCH)
        .expect("Time went backward")
        .as_secs() as u32; // Hi from the past, how's 2038?
    let packet = ServerMessage::new(SmsgQueryTimeResponse {
        seconds_since_epoch,
        seconds_until_daily_quests_reset: 0, // TODO: Change this when implementing daily quests
    });

    let session = session.lock().await;
    packet
        .send(&session.socket, &session.encryption)
        .await
        .unwrap();

    trace!("Sent SMSG_QUERY_TIME_RESPONSE");
}
