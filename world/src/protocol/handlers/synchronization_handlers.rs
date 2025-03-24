use std::time::{SystemTime, UNIX_EPOCH};

use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_ping(PacketHandlerArgs { session, data, .. }: PacketHandlerArgs) {
        let cmsg_ping: CmsgPing = ClientMessage::read_as(data).unwrap();

        session.update_client_latency(cmsg_ping.latency);

        let packet = ServerMessage::new(SmsgPong {
            ping: cmsg_ping.ping,
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_query_time(PacketHandlerArgs { session, .. }: PacketHandlerArgs) {
        let now = SystemTime::now();
        let seconds_since_epoch = now
            .duration_since(UNIX_EPOCH)
            .expect("Time went backward")
            .as_secs() as u32; // Hi from the past, how's 2038?
        let packet = ServerMessage::new(SmsgQueryTimeResponse {
            seconds_since_epoch,
            seconds_until_daily_quests_reset: 0, // TODO: Change this when implementing daily quests
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_time_sync_resp(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg_time_sync_resp: CmsgTimeSyncResp = ClientMessage::read_as(data).unwrap();
        session.handle_time_sync_resp(
            cmsg_time_sync_resp.counter,
            cmsg_time_sync_resp.ticks,
            world_context.game_time().as_millis() as u32,
        );
    }
}
