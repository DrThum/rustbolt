use binrw::{io::Cursor, BinWriterExt, NullString};
use bytemuck::cast_slice;
use miniz_oxide::deflate::CompressionLevel;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use std::{
    sync::{atomic::AtomicU32, Arc},
    time::Duration,
};

use log::trace;
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{self, error::SendError, Sender},
        Mutex,
    },
    task::JoinHandle,
};
use wow_srp::tbc_header::HeaderCrypto;

use crate::{
    entities::{object_guid::ObjectGuid, player::Player},
    game::{map_manager::MapKey, world_context::WorldContext},
    protocol::{
        opcodes::Opcode,
        packets::{
            FactionInit, MovementInfo, SmsgActionButtons, SmsgAttackStop, SmsgCreateObject,
            SmsgDestroyObject, SmsgInitialSpells, SmsgInitializeFactions, SmsgMessageChat,
            SmsgTimeSyncReq, SmsgUpdateObject,
        },
        server::{ServerMessage, ServerMessageHeader, ServerMessagePayload},
    },
    shared::constants::{
        ChatMessageType, Language, MAX_VISIBLE_REPUTATIONS, PLAYER_MAX_ACTION_BUTTONS,
    },
    WorldSocketError,
};

use super::world_socket::WorldSocket;

#[derive(PartialEq, Eq)]
pub enum WorldSessionState {
    OnCharactersList,
    InWorld,
}

pub struct WorldSession {
    socket: WorldSocket,
    tx: Sender<(ServerMessageHeader, Vec<u8>)>,
    pub account_id: u32,
    pub state: parking_lot::RwLock<WorldSessionState>,
    pub player: Arc<parking_lot::RwLock<Player>>,
    client_latency: AtomicU32,
    server_time_sync: Mutex<TimeSync>,
    time_sync_handle: Mutex<Option<JoinHandle<()>>>,
    known_guids: parking_lot::RwLock<Vec<ObjectGuid>>,
}

impl WorldSession {
    pub async fn new(
        socket: TcpStream,
        encryption: HeaderCrypto,
        account_id: u32,
        world_context: Arc<WorldContext>,
    ) -> Arc<WorldSession> {
        let (read_half, write_half) = tokio::io::split(socket);

        let read_half = Arc::new(Mutex::new(read_half));
        let write_half = Arc::new(Mutex::new(write_half));
        let encryption = Arc::new(Mutex::new(encryption));

        let (tx, rx) = mpsc::channel::<(ServerMessageHeader, Vec<u8>)>(50);

        let socket = WorldSocket::new(write_half, read_half, encryption, account_id, rx);

        let session = Arc::new(WorldSession {
            socket,
            tx,
            account_id,
            state: parking_lot::RwLock::new(WorldSessionState::OnCharactersList),
            player: Arc::new(parking_lot::RwLock::new(Player::new())),
            client_latency: AtomicU32::new(0),
            server_time_sync: Mutex::new(TimeSync {
                server_counter: 0,
                server_last_sync_ticks: world_context.game_time().as_millis() as u32,
                client_counter: 0,
                client_last_sync_ticks: 0,
            }),
            time_sync_handle: Mutex::new(None),
            known_guids: parking_lot::RwLock::new(Vec::new()),
        });

        session
    }

    pub async fn shutdown(&self, conn: &mut PooledConnection<SqliteConnectionManager>) {
        self.cleanup_on_world_leave(conn).await;
        self.socket.shutdown().await;
    }

    pub async fn cleanup_on_world_leave(
        &self,
        conn: &mut PooledConnection<SqliteConnectionManager>,
    ) {
        if let Some(handle) = self.time_sync_handle.lock().await.take() {
            handle.abort();
        }

        if self.is_in_world() {
            {
                let mut player = self.player.write();
                let transaction = conn.transaction().unwrap();
                player.save(&transaction).unwrap();
                transaction.commit().unwrap();
            }

            {
                let mut guard = self.known_guids.write();
                guard.clear();
            }
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

    pub fn is_in_world(&self) -> bool {
        // TODO: There might be more states here in the future (BeingTeleportedNear, BeingTeleportedFar?)
        *self.state.read() == WorldSessionState::InWorld
    }

    pub async fn handle_time_sync_resp(
        &self,
        client_counter: u32,
        client_ticks: u32,
        server_ticks: u32,
    ) {
        let mut time_sync = self.server_time_sync.lock().await;

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
        let tx = self.tx.clone();
        futures::executor::block_on(async move {
            // Handle::current().block_on(async move {
            let payload = packet.encode_payload().expect("failed to encode payload");
            let channel_payload = if OPCODE == Opcode::SmsgUpdateObject as u16 && payload.len() > 50
            {
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

            tx.send(channel_payload).await
        })
    }

    pub fn send_movement(
        &self,
        opcode: Opcode,
        origin_guid: &ObjectGuid,
        movement_info: &MovementInfo,
    ) -> Result<(), SendError<(ServerMessageHeader, Vec<u8>)>> {
        let tx = self.tx.clone();
        futures::executor::block_on(async move {
            let mut writer = Cursor::new(Vec::new());
            writer.write_le(&origin_guid.as_packed()).unwrap();
            writer.write_le(movement_info).unwrap();
            let payload = writer.get_ref().clone();

            let header = ServerMessageHeader {
                size: payload.len() as u16 + 2, // + 2 for the opcode size
                opcode: opcode as u16,
            };

            tx.send((header, payload)).await
        })
    }

    pub async fn process_incoming_packet(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
    ) -> Result<(), WorldSocketError> {
        let client_message = session.socket.read_packet().await?;

        // Process the packet on the world's runtime
        let context_clone = world_context.clone();
        world_context.clone().map_manager.runtime.spawn(async move {
            let handler = context_clone
                .opcode_handler
                .get_handler(client_message.header.opcode);

            handler(
                session.clone(),
                world_context.clone(),
                client_message.payload,
            )
            .await;
        });

        Ok(())
    }

    async fn schedule_time_sync(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                {
                    let mut time_sync = session.server_time_sync.lock().await;

                    let smsg_time_sync_req = ServerMessage::new(SmsgTimeSyncReq {
                        sync_counter: time_sync.server_counter,
                    });
                    session.send(&smsg_time_sync_req).unwrap();

                    time_sync.server_counter += 1;
                    time_sync.server_last_sync_ticks = world_context.game_time().as_millis() as u32;
                }

                interval.tick().await;
            }
        })
    }

    // TODO: Reset time sync after each teleport
    pub async fn reset_time_sync(session: Arc<WorldSession>, world_context: Arc<WorldContext>) {
        let mut guard = session.time_sync_handle.lock().await;
        if let Some(handle) = guard.take() {
            handle.abort();
        }

        {
            let mut time_sync = session.server_time_sync.lock().await;
            time_sync.reset();
        }

        let jh = WorldSession::schedule_time_sync(session.clone(), world_context).await;
        *guard = Some(jh);
    }

    pub fn get_current_map(&self) -> Option<MapKey> {
        self.player.read().current_map()
    }

    pub fn set_map(&self, key: MapKey) {
        self.player.write().set_map(key);
    }

    pub async fn send_initial_spells(&self) {
        let spells: Vec<u32> = self.player.read().spells().clone();
        let packet = ServerMessage::new(SmsgInitialSpells::new(spells, Vec::new() /* TODO */));
        self.send(&packet).unwrap();
    }

    pub async fn send_initial_action_buttons(&self) {
        let action_buttons = self.player.read().action_buttons().clone();

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

    pub async fn send_initial_reputations(&self) {
        let faction_standings = self.player.read().faction_standings().clone();

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

    pub async fn build_chat_packet(
        &self,
        message_type: ChatMessageType,
        language: Language,
        target_guid: Option<&ObjectGuid>,
        message: NullString,
    ) -> SmsgMessageChat {
        SmsgMessageChat {
            message_type,
            language,
            sender_guid: self.player.read().guid().raw(),
            unk: 0,
            target_guid: target_guid.map_or(0, |g| g.raw()),
            message_len: message.len() as u32 + 1,
            message,
            chat_tag: 0, // TODO: Implement chat tags (GM, AFK, DND)
        }
    }

    fn add_known_guid(&self, guid: &ObjectGuid) {
        self.known_guids.write().push(guid.clone());
    }

    fn remove_known_guid(&self, guid: &ObjectGuid) {
        self.known_guids.write().retain(|g| g != guid);
    }

    pub async fn is_guid_known(&self, guid: &ObjectGuid) -> bool {
        if guid == self.player.read().guid() {
            return true;
        }

        self.known_guids.read().contains(guid)
    }

    pub async fn create_entity(&self, guid: &ObjectGuid, payload: SmsgCreateObject) {
        let packet = ServerMessage::new(payload);

        self.send(&packet).unwrap();
        self.add_known_guid(guid);
    }

    pub fn update_entity(&self, payload: SmsgUpdateObject) {
        let packet = ServerMessage::new(payload);

        self.send(&packet).unwrap();
    }

    pub async fn destroy_entity(&self, guid: &ObjectGuid) {
        if self.is_guid_known(guid).await {
            let packet = ServerMessage::new(SmsgDestroyObject { guid: guid.raw() });

            self.send(&packet).unwrap();
            self.remove_known_guid(guid);
        }
    }

    pub async fn send_attack_stop(&self, target_guid: Option<ObjectGuid>) {
        let packet = ServerMessage::new(SmsgAttackStop {
            player_guid: self.player.read().guid().as_packed(),
            enemy_guid: target_guid.unwrap_or(ObjectGuid::zero()).as_packed(),
            unk: 0,
        });

        self.send(&packet).unwrap();
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
