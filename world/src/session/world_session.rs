use std::sync::{atomic::AtomicU32, Arc};

use log::trace;
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock},
};
use wow_srp::tbc_header::HeaderCrypto;

use crate::{
    entities::player::Player,
    game::world_context::WorldContext,
    protocol::{
        opcodes::Opcode,
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
}

impl WorldSession {
    pub fn new(socket: TcpStream, encryption: HeaderCrypto, account_id: u32) -> WorldSession {
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

        WorldSession {
            socket,
            account_id,
            player: Arc::new(RwLock::new(Player::new())),
            client_latency: AtomicU32::new(0),
        }
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
}
