use std::sync::Arc;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;

impl OpcodeHandler {
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
