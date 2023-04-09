use std::{
    sync::{atomic::AtomicU32, Arc},
    time::Duration,
};

use log::trace;
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock},
    task::JoinHandle,
};
use wow_srp::tbc_header::HeaderCrypto;

use crate::{
    entities::player::Player,
    game::world_context::WorldContext,
    protocol::{
        opcodes::Opcode,
        packets::SmsgTimeSyncReq,
        server::{ServerMessage, ServerMessagePayload},
    },
    WorldSocketError,
};

use super::world_socket::WorldSocket;

pub struct WorldSession {
    socket: WorldSocket,
    pub account_id: u32,
    pub player: Arc<RwLock<Player>>,
    client_latency: AtomicU32,
    server_time_sync: Mutex<TimeSync>,
    pub time_sync_handle: Mutex<Option<JoinHandle<()>>>,
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

        let socket = WorldSocket {
            write_half,
            read_half,
            encryption,
            account_id,
        };

        let session = Arc::new(WorldSession {
            socket,
            account_id,
            player: Arc::new(RwLock::new(Player::new())),
            client_latency: AtomicU32::new(0),
            server_time_sync: Mutex::new(TimeSync {
                server_counter: 0,
                server_last_sync_ticks: world_context.game_time().as_millis() as u32,
                client_counter: 0,
                client_last_sync_ticks: 0,
            }),
            time_sync_handle: Mutex::new(None),
        });

        session
    }

    pub async fn shutdown(&self) {
        self.socket.shutdown().await;
    }

    pub fn client_latency(&self) -> u32 {
        self.client_latency
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn update_client_latency(&self, latency: u32) {
        self.client_latency
            .store(latency, std::sync::atomic::Ordering::Relaxed);
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

    pub async fn send<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE>>(
        &self,
        packet: ServerMessage<OPCODE, Payload>,
    ) -> Result<(), binrw::Error> {
        let mut socket = self.socket.write_half.lock().await;
        let mut encryption = self.socket.encryption.lock().await;

        trace!("Sending {:?} ({:#X})", Opcode::n(OPCODE).unwrap(), OPCODE);
        packet.send(&mut socket, &mut encryption).await
    }

    pub async fn process_incoming_packet(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
    ) -> Result<(), WorldSocketError> {
        let client_message = session.socket.read_packet().await?;
        let handler = world_context
            .opcode_handler
            .get_handler(client_message.header.opcode);

        handler(
            session.clone(),
            world_context.clone(),
            client_message.payload,
        )
        .await;

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
                    session.send(smsg_time_sync_req).await.unwrap();

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
