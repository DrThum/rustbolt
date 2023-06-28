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
            let player_guid = session.player_guid().unwrap();

            // TODO: Validate movement (unitBeingMoved guid)
            // TODO: Validate position
            // TODO: Handle fall if Opcode == MsgMoveFallLand

            // Register new position
            {
                session.current_map().unwrap().update_player_position(
                    &player_guid,
                    session.clone(),
                    &movement_info.position,
                    world_context.clone(),
                );
            }

            if opcode == Opcode::MsgMoveJump {
                let map = session.current_map().unwrap();
                let height =
                    map.get_terrain_height(movement_info.position.x, movement_info.position.y);
                println!(
                    "Height: {:?} (client reported {:?})",
                    height, movement_info.position
                );
            }

            // Broadcast to nearby players
            session
                .current_map()
                .unwrap()
                .broadcast_movement(&player_guid, opcode, &movement_info);
        }

        Box::new(move |session, ctx, data| handle_movement(opcode, session, ctx, data))
            as PacketHandler
    }
}
