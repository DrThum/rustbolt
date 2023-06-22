use std::sync::Arc;

use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;
use crate::shared::constants::QuestGiverStatus;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_quest_giver_status_query(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverStatusQuery = ClientMessage::read_as(data).unwrap();

        let packet = ServerMessage::new(SmsgQuestGiverStatus {
            guid: cmsg.guid,
            status: QuestGiverStatus::Available,
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_quest_giver_hello(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgQuestGiverHello = ClientMessage::read_as(data).unwrap();

        session.current_map().unwrap().world().run(|| {

        });
    }
}
