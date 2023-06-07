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
        fn handle_movement(
            opcode: Opcode,
            session: Arc<WorldSession>,
            world_context: Arc<WorldContext>,
            data: Vec<u8>,
        ) {
            let movement_info: MovementInfo = ClientMessage::read_as(data).unwrap();

            // TODO: Validate movement (unitBeingMoved guid)
            // TODO: Validate position
            // TODO: Handle fall if Opcode == MsgMoveFallLand

            // Register new position
            {
                world_context.map_manager.update_player_position(
                    world_context.clone(),
                    session.clone(),
                    &movement_info.position,
                );

                let mut player = session.player.write();
                player.set_position(&movement_info.position);
            }

            // Broadcast to nearby players
            world_context.map_manager.broadcast_movement(
                session.player.clone(),
                opcode,
                &movement_info,
            );
        }

        Box::new(move |session, ctx, data| handle_movement(opcode, session, ctx, data))
            as PacketHandler
    }
}
