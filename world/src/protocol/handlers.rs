use binrw::NullString;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::entities::player::Player;
use crate::entities::update::UpdatableEntity;
use crate::game::world_context::WorldContext;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::repositories::character::CharacterRepository;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;
use crate::shared::response_codes::ResponseCodes;

use super::client::ClientMessage;

impl OpcodeHandler {
    pub(crate) async fn handle_cmsg_char_create(
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

        session.send(packet).await.unwrap();
    }

    pub(crate) async fn handle_cmsg_char_enum(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        _data: Vec<u8>,
    ) {
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

        session.send(packet).await.unwrap();
    }

    pub(crate) async fn handle_cmsg_char_delete(
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

        session.send(packet).await.unwrap();
    }

    pub(crate) async fn handle_cmsg_player_login(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_player_login: CmsgPlayerLogin = ClientMessage::read_as(data).unwrap();

        let account_id = session.account_id;
        let conn = world_context.database.characters.get().unwrap();

        {
            let mut player = session.player.write().await;
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

        session.send(msg_set_dungeon_difficulty).await.unwrap();

        {
            let player = session.player.read().await;
            let player_position = player.position();
            let smsg_login_verify_world = ServerMessage::new(SmsgLoginVerifyWorld {
                map: player_position.map,
                position_x: player_position.x,
                position_y: player_position.y,
                position_z: player_position.z,
                orientation: player_position.o,
            });

            session.send(smsg_login_verify_world).await.unwrap();
        }

        let smsg_account_data_times =
            ServerMessage::new(SmsgAccountDataTimes { data: [0_u32; 32] });

        session.send(smsg_account_data_times).await.unwrap();

        let smsg_feature_system_status = ServerMessage::new(SmsgFeatureSystemStatus {
            unk: 2,
            voice_chat_enabled: 0,
        });

        session.send(smsg_feature_system_status).await.unwrap();

        let smsg_motd = ServerMessage::new(SmsgMotd {
            line_count: 1,
            message: NullString::from("MOTD"), // TODO: store this in config file
        });

        session.send(smsg_motd).await.unwrap();

        let smsg_set_rest_start = ServerMessage::new(SmsgSetRestStart { rest_start: 0 });

        session.send(smsg_set_rest_start).await.unwrap();

        // TODO
        let smsg_bindpointupdate = ServerMessage::new(SmsgBindpointupdate {
            homebind_x: -8953.95,
            homebind_y: 521.019,
            homebind_z: 96.5399,
            homebind_map_id: 0,
            homebind_area_id: 85,
        });

        session.send(smsg_bindpointupdate).await.unwrap();

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

        session.send(smsg_tutorial_flags).await.unwrap();

        let smsg_login_settimespeed = ServerMessage::new(SmsgLoginSettimespeed {
            timestamp: 0, // Maybe not zero?
            game_speed: 0.01666667,
        });

        session.send(smsg_login_settimespeed).await.unwrap();

        {
            let player = session.player.read().await;
            let update_data = player.get_create_data(player.guid().raw(), world_context.clone());
            let smsg_update_object = ServerMessage::new(SmsgUpdateObject {
                updates_count: update_data.len() as u32,
                has_transport: false,
                updates: update_data,
            });

            session.send(smsg_update_object).await.unwrap();
        }

        let smsg_init_world_states = ServerMessage::new(SmsgInitWorldStates {
            map_id: 0,
            zone_id: 85,
            area_id: 154, // Deathknell
            block_count: 0,
        });

        session.send(smsg_init_world_states).await.unwrap();

        // TODO:
        // - move this to WorldSession and store the counter
        // - send this every 10 seconds
        // - increment the counter each time the packet is sent
        // - implement CmsgTimeSyncResp and check the difference between client and server counters
        // - LATER: reset the counter after every teleport (is that really necessary?)
        //
        // implement WorldSession::reset_time_sync() and WorldSession::send_time_sync()
        // How to handle the timer?
        let smsg_time_sync_req = ServerMessage::new(SmsgTimeSyncReq { sync_counter: 0 });

        session.send(smsg_time_sync_req).await.unwrap();
    }

    pub async fn unhandled(
        _session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        _data: Vec<u8>,
    ) {
    }

    pub(crate) async fn handle_cmsg_realm_split(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_realm_split: CmsgRealmSplit = ClientMessage::read_as(data).unwrap();

        let packet = ServerMessage::new(SmsgRealmSplit {
            client_state: cmsg_realm_split.client_state,
            realm_state: 0x00,
            split_date: binrw::NullString::from("01/01/01"),
        });

        session.send(packet).await.unwrap();
    }

    pub(crate) async fn handle_cmsg_ping(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_ping: CmsgPing = ClientMessage::read_as(data).unwrap();

        session.update_client_latency(cmsg_ping.latency);

        let packet = ServerMessage::new(SmsgPong {
            ping: cmsg_ping.ping,
        });

        session.send(packet).await.unwrap();
    }

    pub(crate) async fn handle_cmsg_update_account_data(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_update_account_data: CmsgUpdateAccountData = ClientMessage::read_as(data).unwrap();

        let packet = ServerMessage::new(SmsgUpdateAccountData {
            account_data_id: cmsg_update_account_data.account_data_id,
            data: 0,
        });

        session.send(packet).await.unwrap();
    }

    pub(crate) async fn handle_cmsg_logout_request(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        _data: Vec<u8>,
    ) {
        let packet = ServerMessage::new(SmsgLogoutResponse {
            reason: 0,
            is_instant_logout: 1,
        });

        session.send(packet).await.unwrap();

        let packet = ServerMessage::new(SmsgLogoutComplete {});

        session.send(packet).await.unwrap();
    }

    pub(crate) async fn handle_cmsg_item_query_single(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_item_query_single: CmsgItemQuerySingle = ClientMessage::read_as(data).unwrap();

        let packet = if let Some(item) = world_context
            .data_store
            .get_item_template(cmsg_item_query_single.item_id)
        {
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

        session.send(packet).await.unwrap();
    }

    pub(crate) async fn handle_cmsg_name_query(
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

        session.send(packet).await.unwrap();
    }

    pub(crate) async fn handle_cmsg_query_time(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        _data: Vec<u8>,
    ) {
        let now = SystemTime::now();
        let seconds_since_epoch = now
            .duration_since(UNIX_EPOCH)
            .expect("Time went backward")
            .as_secs() as u32; // Hi from the past, how's 2038?
        let packet = ServerMessage::new(SmsgQueryTimeResponse {
            seconds_since_epoch,
            seconds_until_daily_quests_reset: 0, // TODO: Change this when implementing daily quests
        });

        session.send(packet).await.unwrap();
    }
}
