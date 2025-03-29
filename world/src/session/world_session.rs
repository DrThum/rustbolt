use binrw::{io::Cursor, BinWriterExt, NullString};
use bytemuck::cast_slice;
use chrono::{Datelike, Timelike};
use miniz_oxide::deflate::CompressionLevel;
use parking_lot::RwLock;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use shipyard::{EntityId, View, ViewMut};
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use log::{error, trace};
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{self, error::SendError, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
};
use wow_srp::tbc_header::HeaderCrypto;

use crate::{
    ecs::components::powers::Powers,
    entities::{object_guid::ObjectGuid, player::Player, position::WorldPosition},
    game::{map::Map, world_context::WorldContext},
    protocol::{
        client::ClientMessage,
        opcodes::Opcode,
        packets::{
            FactionInit, MovementInfo, SmsgActionButtons, SmsgAttackStop, SmsgBindpointupdate,
            SmsgCreateObject, SmsgDestroyObject, SmsgInitWorldStates, SmsgInitialSpells,
            SmsgInitializeFactions, SmsgLoginSetTimeSpeed, SmsgMessageChat, SmsgSetRestStart,
            SmsgTimeSyncReq, SmsgTutorialFlags, SmsgUpdateObject,
        },
        server::{ServerMessage, ServerMessageHeader, ServerMessagePayload},
    },
    repositories::character::CharacterRepository,
    shared::constants::{
        ChatMessageType, Language, MAX_VISIBLE_REPUTATIONS, PLAYER_MAX_ACTION_BUTTONS,
    },
    WorldSocketError,
};

use super::{
    opcode_handler::OpcodeProcessingMode, opcode_handler::PacketHandlerArgs,
    world_socket::WorldSocket,
};

#[derive(PartialEq, Eq)]
pub enum WorldSessionState {
    OnCharactersList,
    InWorld,
    InMapTransfer,
}

pub struct WorldSession {
    socket: WorldSocket,
    session_to_socket_tx: UnboundedSender<(ServerMessageHeader, Vec<u8>)>,
    pub account_id: u32,
    pub state: RwLock<WorldSessionState>,
    current_map: RwLock<Option<Arc<Map>>>,
    player_entity_id: RwLock<Option<EntityId>>,
    player_guid: RwLock<Option<ObjectGuid>>,
    client_latency: AtomicU32,
    server_time_sync: parking_lot::Mutex<TimeSync>,
    time_sync_handle: parking_lot::Mutex<Option<JoinHandle<()>>>,
    known_guids: RwLock<Vec<ObjectGuid>>,
}

impl WorldSession {
    pub fn new(
        socket: TcpStream,
        encryption: HeaderCrypto,
        account_id: u32,
        world_context: Arc<WorldContext>,
    ) -> Arc<WorldSession> {
        let (read_half, write_half) = tokio::io::split(socket);

        let read_half = Arc::new(Mutex::new(read_half));
        let write_half = Arc::new(Mutex::new(write_half));
        let encryption = Arc::new(Mutex::new(encryption));

        let (session_to_socket_tx, session_to_socket_rx) =
            mpsc::unbounded_channel::<(ServerMessageHeader, Vec<u8>)>();

        let (socket_to_session_tx, mut socket_to_session_rx) =
            mpsc::unbounded_channel::<ClientMessage>();

        let socket = WorldSocket::new(
            write_half,
            read_half,
            encryption,
            account_id,
            session_to_socket_rx,
            socket_to_session_tx,
        );

        let session = Arc::new(WorldSession {
            socket,
            session_to_socket_tx,
            account_id,
            state: RwLock::new(WorldSessionState::OnCharactersList),
            current_map: RwLock::new(None),
            player_entity_id: RwLock::new(None),
            player_guid: RwLock::new(None),
            client_latency: AtomicU32::new(0),
            server_time_sync: parking_lot::Mutex::new(TimeSync {
                server_counter: 0,
                server_last_sync_ticks: world_context.game_time().as_millis() as u32,
                client_counter: 0,
                client_last_sync_ticks: 0,
            }),
            time_sync_handle: parking_lot::Mutex::new(None),
            known_guids: RwLock::new(Vec::new()),
        });

        let world_context_clone = world_context.clone();
        let session_clone = session.clone();
        tokio::spawn(async move {
            while let Some(client_message) = socket_to_session_rx.recv().await {
                let (processing_mode, handler) = world_context_clone
                    .opcode_handler
                    .get_handler(client_message.header.opcode);

                match processing_mode {
                    OpcodeProcessingMode::ProcessInMap => {
                        if let Some(map) = session_clone.current_map() {
                            map.queue_packet(session_clone.clone(), client_message);
                        } else {
                            error!("received a packet that must be processed on the map but player is not on a map! ({:?} - {:#X})", Opcode::n(client_message.header.opcode).unwrap(), client_message.header.opcode)
                        }
                    }
                    OpcodeProcessingMode::ProcessImmediately => {
                        handler(PacketHandlerArgs {
                            session: session_clone.clone(),
                            world_context: world_context_clone.clone(),
                            data: client_message.payload,
                        });
                    }
                    OpcodeProcessingMode::Ignore => (),
                }
            }
        });

        session
    }

    pub fn shutdown(
        &self,
        conn: &mut PooledConnection<SqliteConnectionManager>,
        world_context: Arc<WorldContext>,
    ) {
        self.cleanup_on_world_leave(conn, world_context);
        self.socket.shutdown();
    }

    fn cleanup_on_world_leave(
        &self,
        conn: &mut PooledConnection<SqliteConnectionManager>,
        world_context: Arc<WorldContext>,
    ) {
        if let Some(handle) = self.time_sync_handle.lock().take() {
            handle.abort();
        }

        if let Some(map) = self.current_map() {
            if let Some(entity_id) = self.player_entity_id() {
                map.world().run(
                    |mut vm_player: ViewMut<Player>,
                     v_wpos: View<WorldPosition>,
                     v_powers: View<Powers>| {
                        let transaction = conn.transaction().unwrap();

                        CharacterRepository::save_to_db(
                            &transaction,
                            &mut vm_player[entity_id],
                            &v_powers[entity_id],
                            &v_wpos[entity_id],
                        )
                        .unwrap();
                        transaction.commit().unwrap();
                    },
                );
            }

            map.remove_player_on_logout(&self.player_guid.read().unwrap());

            self.known_guids.write().clear();
            self.current_map.write().take();
            self.player_entity_id.write().take();
            self.player_guid.write().take();

            world_context
                .session_holder
                .remove_session(&self.account_id);
        }
    }

    pub fn client_latency(&self) -> u32 {
        self.client_latency
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn update_client_latency(&self, latency: u32) {
        self.client_latency
            .store(latency, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn handle_time_sync_resp(&self, client_counter: u32, client_ticks: u32, server_ticks: u32) {
        let mut time_sync = self.server_time_sync.lock();

        let counter_ok = client_counter == time_sync.server_counter - 1;
        let server_ticks = client_ticks + (server_ticks - time_sync.server_last_sync_ticks);

        trace!(
            "Time sync:\n\
            \tCounters: Server: {} / Client: {} ({})\n\
            \tTime since last client sync: {:?}\n\
            \tTime difference: {:?} (client latency: {})",
            time_sync.server_counter,
            client_counter,
            if counter_ok { "OK" } else { "NOK" },
            Duration::from_millis((client_ticks - time_sync.client_last_sync_ticks) as u64),
            Duration::from_millis((server_ticks - client_ticks) as u64),
            self.client_latency(),
        );

        time_sync.client_counter = client_counter;
        time_sync.client_last_sync_ticks = client_ticks;
    }

    pub fn send<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE>>(
        &self,
        packet: &ServerMessage<OPCODE, Payload>,
    ) -> Result<(), SendError<(ServerMessageHeader, Vec<u8>)>> {
        let tx = self.session_to_socket_tx.clone();
        let payload = packet.encode_payload().expect("failed to encode payload");
        let channel_payload = if OPCODE == Opcode::SmsgUpdateObject as u16 && payload.len() > 50 {
            // Change to SMSG_COMPRESSED_UPDATE_OBJECT and compress the payload
            let uncompressed_size = payload.len();
            let compressed_payload: Vec<u8> = miniz_oxide::deflate::compress_to_vec_zlib(
                &payload,
                CompressionLevel::DefaultLevel as u8,
            );

            let header = ServerMessageHeader {
                size: compressed_payload.len() as u16 + 2 + 4, /* + 2 for opcode + 4 for uncompressed_size */
                opcode: Opcode::SmsgCompressedUpdateObject as u16,
            };

            let payload: Vec<u32> = vec![uncompressed_size as u32];
            let mut payload: Vec<u8> = cast_slice(&payload).to_vec();
            payload.extend(compressed_payload);
            (header, payload)
        } else {
            let header = ServerMessageHeader {
                size: payload.len() as u16 + 2, // + 2 for the opcode size
                opcode: OPCODE,
            };

            (header, payload)
        };
        tx.send(channel_payload)
    }

    pub fn send_movement(
        &self,
        opcode: Opcode,
        origin_guid: &ObjectGuid,
        movement_info: &MovementInfo,
    ) -> Result<(), SendError<(ServerMessageHeader, Vec<u8>)>> {
        let tx = self.session_to_socket_tx.clone();
        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&origin_guid.as_packed()).unwrap();
        writer.write_le(movement_info).unwrap();
        let payload = writer.get_ref().clone();

        let header = ServerMessageHeader {
            size: payload.len() as u16 + 2, // + 2 for the opcode size
            opcode: opcode as u16,
        };

        tx.send((header, payload))
    }

    pub async fn process_incoming_packet(
        session: Arc<WorldSession>,
    ) -> Result<(), WorldSocketError> {
        let client_message = session.socket.read_packet().await?;
        if let Err(e) = session.socket.queue_client_message(client_message) {
            return Err(e.into());
        }

        Ok(())
    }

    fn schedule_time_sync(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                Self::send_time_sync_req(session.clone(), world_context.clone());
                interval.tick().await;
            }
        })
    }

    fn send_time_sync_req(session: Arc<WorldSession>, world_context: Arc<WorldContext>) {
        let mut time_sync = session.server_time_sync.lock();

        let smsg_time_sync_req = ServerMessage::new(SmsgTimeSyncReq {
            sync_counter: time_sync.server_counter,
        });
        session.send(&smsg_time_sync_req).unwrap();

        time_sync.server_counter += 1;
        time_sync.server_last_sync_ticks = world_context.game_time().as_millis() as u32;
    }

    fn reset_time_sync(session: Arc<WorldSession>, world_context: Arc<WorldContext>) {
        session.server_time_sync.lock().reset();

        let mut guard = session.time_sync_handle.lock();
        if guard.is_some() {
            // If guard is_some, it means that the time sync is already scheduled, meaning that
            // we are in the case of a map transfer, not a character login. In that case, we need
            // to send a new time sync request right away, otherwise it might take up to 10 seconds
            // to get one, and until the client has received a time sync request, it won't allow
            // the character to move.
            Self::send_time_sync_req(session.clone(), world_context.clone());
            return;
        }

        let jh = WorldSession::schedule_time_sync(session.clone(), world_context);
        *guard = Some(jh);
    }

    pub fn current_map(&self) -> Option<Arc<Map>> {
        self.current_map.read().as_ref().cloned()
    }

    pub fn set_player_guid(&self, guid: ObjectGuid) {
        self.player_guid.write().replace(guid);
    }

    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.player_guid.read().as_ref().cloned()
    }

    pub fn set_player_entity_id(&self, entity_id: EntityId) {
        self.player_entity_id.write().replace(entity_id);
    }

    pub fn player_entity_id(&self) -> Option<EntityId> {
        self.player_entity_id.read().as_ref().cloned()
    }

    pub fn set_map(&self, map: Arc<Map>) {
        self.current_map.write().replace(map);
    }

    pub fn send_initial_spells(&self, player: &Player) {
        let spells: Vec<u32> = player.spells().to_vec();
        let packet = ServerMessage::new(SmsgInitialSpells::new(spells, Vec::new() /* TODO */));
        self.send(&packet).unwrap();
    }

    pub fn send_initial_action_buttons(&self, player: &Player) {
        let action_buttons = player.action_buttons().clone();

        let mut buttons_packed: Vec<u32> = Vec::new();
        for index in 0..PLAYER_MAX_ACTION_BUTTONS {
            let packed = action_buttons
                .get(&index)
                .map_or(0, |button| button.packed());

            buttons_packed.push(packed);
        }

        let packet = ServerMessage::new(SmsgActionButtons { buttons_packed });

        self.send(&packet).unwrap();
    }

    pub fn send_initial_reputations(&self, player: &Player) {
        let faction_standings = player.faction_standings().clone();

        let mut factions: Vec<FactionInit> = Vec::with_capacity(MAX_VISIBLE_REPUTATIONS);
        for index in 0..MAX_VISIBLE_REPUTATIONS {
            let faction_init =
                if let Some(faction_standing) = faction_standings.get(&(index as u32)) {
                    FactionInit {
                        flags: faction_standing.flags as u8,
                        standing: faction_standing.db_standing as u32,
                    }
                } else {
                    FactionInit {
                        flags: 0,
                        standing: 0,
                    }
                };

            factions.push(faction_init);
        }

        let packet = ServerMessage::new(SmsgInitializeFactions {
            unk: 0x80,
            factions,
        });

        self.send(&packet).unwrap();
    }

    pub fn build_chat_packet(
        &self,
        message_type: ChatMessageType,
        language: Language,
        target_guid: Option<&ObjectGuid>,
        message: NullString,
    ) -> SmsgMessageChat {
        SmsgMessageChat::build(
            message_type,
            language,
            self.player_guid.read().as_ref(),
            target_guid,
            message,
        )
    }

    fn add_known_guid(&self, guid: &ObjectGuid) {
        self.known_guids.write().push(*guid);
    }

    fn remove_known_guid(&self, guid: &ObjectGuid) {
        self.known_guids.write().retain(|g| g != guid);
    }

    pub fn is_guid_known(&self, guid: &ObjectGuid) -> bool {
        if *guid == self.player_guid.read().unwrap() {
            return true;
        }

        self.known_guids.read().contains(guid)
    }

    pub fn known_guids(&self) -> Vec<ObjectGuid> {
        self.known_guids.read().clone()
    }

    pub fn create_entity(&self, guid: &ObjectGuid, payload: SmsgCreateObject) {
        let packet = ServerMessage::new(payload);

        self.send(&packet).unwrap();
        self.add_known_guid(guid);
    }

    pub fn update_entity(&self, payload: SmsgUpdateObject) {
        let packet = ServerMessage::new(payload);

        self.send(&packet).unwrap();
    }

    pub fn destroy_entity(&self, guid: &ObjectGuid) {
        if self.is_guid_known(guid) {
            let packet = ServerMessage::new(SmsgDestroyObject { guid: guid.raw() });

            self.send(&packet).unwrap();
            self.remove_known_guid(guid);
        }
    }

    pub fn send_attack_stop(&self, target_guid: Option<ObjectGuid>) {
        let packet = ServerMessage::new(SmsgAttackStop {
            attacker_guid: self.player_guid.read().unwrap().as_packed(),
            enemy_guid: target_guid.unwrap_or(ObjectGuid::zero()).as_packed(),
            unk: 0,
        });

        self.send(&packet).unwrap();
    }

    pub fn send_system_message(&self, message: &str) {
        let packet = ServerMessage::new(SmsgMessageChat::build(
            ChatMessageType::System,
            Language::Universal,
            None,
            None,
            NullString::from(message),
        ));

        self.send(&packet).unwrap();
    }

    pub fn send_error_system_message(&self, message: &str) {
        self.send_system_message(format!("|cffff0000Error:|r {message}").as_str())
    }

    pub fn run<T>(&self, f: &dyn Fn(WSRunnableArgs) -> T) -> Option<T> {
        if let Some(map) = self.current_map() {
            if let Some(player_entity_id) = self.player_entity_id() {
                return Some(f(WSRunnableArgs {
                    map,
                    player_entity_id,
                }));
            }
        }

        None
    }

    pub fn set_state(self: Arc<Self>, state: WorldSessionState) {
        let mut session_state = self.state.write();
        *session_state = state;
    }

    // Make nearby GameObjects related to the quest active or inactive depending on player quest
    // status
    pub fn force_refresh_nearby_game_objects(&self, player: &Player) {
        player
            .needs_nearby_game_objects_refresh
            .store(true, Ordering::Relaxed);
    }

    pub fn send_initial_packets_before_add_to_map(self: &Arc<Self>) {
        let smsg_set_rest_start = ServerMessage::new(SmsgSetRestStart { rest_start: 0 });

        self.send(&smsg_set_rest_start).unwrap();

        // TODO
        let smsg_bindpointupdate = ServerMessage::new(SmsgBindpointupdate {
            homebind_x: -8953.95,
            homebind_y: 521.019,
            homebind_z: 96.5399,
            homebind_map_id: 0,
            homebind_area_id: 85,
        });

        self.send(&smsg_bindpointupdate).unwrap();

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

        self.send(&smsg_tutorial_flags).unwrap();

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

        let smsg_login_set_time_speed = ServerMessage::new(SmsgLoginSetTimeSpeed {
            timestamp,
            game_speed: 0.01666667,
        });

        self.send(&smsg_login_set_time_speed).unwrap();

        // FIXME: hardcoded position
        // FIXME: should be sent whenever the player changes zone
        let smsg_init_world_states = ServerMessage::new(SmsgInitWorldStates {
            map_id: 0,
            zone_id: 85,
            area_id: 154, // Deathknell
            block_count: 0,
        });

        self.send(&smsg_init_world_states).unwrap();
    }

    pub fn send_initial_packets_after_add_to_map(
        self: &Arc<Self>,
        world_context: Arc<WorldContext>,
    ) {
        Self::reset_time_sync(self.clone(), world_context);
    }
}

impl PartialEq for WorldSession {
    fn eq(&self, other: &Self) -> bool {
        self.account_id == other.account_id
    }
}

impl std::cmp::Eq for WorldSession {}

impl std::hash::Hash for WorldSession {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.account_id.hash(state);
    }
}

struct TimeSync {
    pub server_counter: u32,
    pub server_last_sync_ticks: u32,
    pub client_counter: u32,
    pub client_last_sync_ticks: u32,
}

impl TimeSync {
    pub fn reset(&mut self) {
        self.server_counter = 0;
        self.server_last_sync_ticks = 0;
        self.client_counter = 0;
        self.client_last_sync_ticks = 0;
    }
}

pub struct WSRunnableArgs {
    pub map: Arc<Map>,
    pub player_entity_id: EntityId,
}
