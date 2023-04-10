use futures::FutureExt;
use std::sync::Arc;

use crate::{
    game::world_context::WorldContext,
    protocol::{client::ClientMessage, opcodes::Opcode, packets::MovementInfo},
    session::{
        opcode_handler::{OpcodeHandler, PacketHandler},
        world_session::WorldSession,
    },
};

impl OpcodeHandler {
    pub fn handle_movement_packet(opcode: Opcode) -> PacketHandler {
        async fn handle_movement(
            _opcode: Opcode,
            _session: Arc<WorldSession>,
            _world_context: Arc<WorldContext>,
            data: Vec<u8>,
        ) {
            let movement_info: MovementInfo = ClientMessage::read_as(data).unwrap();

            // TODO: Validate movement (unitBeingMoved guid)
            // TODO: Validate position
            // TODO: Handle fall if Opcode == MsgMoveFallLand
            println!("Received movement info {:?}", movement_info);
        }

        Box::new(move |session, ctx, data| { handle_movement(opcode, session, ctx, data) }.boxed())
            as PacketHandler
    }
}
