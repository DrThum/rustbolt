use std::sync::Arc;

use shipyard::{View, ViewMut};

use crate::{
    ecs::components::{behavior::Behavior, movement::Movement},
    entities::{creature::Creature, player::Player, position::WorldPosition},
    protocol::{client::ClientMessage, opcodes::Opcode, packets::MovementInfo},
    session::{
        opcode_handler::{OpcodeHandler, PacketHandler},
        world_session::WorldSession,
    },
};

impl OpcodeHandler {
    pub fn handle_movement_packet(opcode: Opcode) -> PacketHandler {
        fn handle_movement(opcode: Opcode, session: Arc<WorldSession>, data: Vec<u8>) {
            let movement_info: MovementInfo = ClientMessage::read_as(data).unwrap();
            let player_guid = session.player_guid().unwrap();

            // TODO: Validate movement (unitBeingMoved guid)
            // TODO: Validate position
            // TODO: Handle fall if Opcode == MsgMoveFallLand

            let map = session.current_map().unwrap();
            // Register new position
            map.world().run(
                |v_movement: View<Movement>,
                 v_player: View<Player>,
                 v_creature: View<Creature>,
                 mut vm_wpos: ViewMut<WorldPosition>,
                 mut vm_behavior: ViewMut<Behavior>| {
                    map.update_entity_position(
                        &player_guid,
                        session.player_entity_id().unwrap(),
                        Some(session.clone()),
                        &movement_info.position,
                        &v_movement,
                        &v_player,
                        &v_creature,
                        &mut vm_wpos,
                        &mut vm_behavior,
                    );
                },
            );

            // Broadcast to nearby players
            map.broadcast_movement(&player_guid, opcode, &movement_info);
        }

        Box::new(move |session, _, data| handle_movement(opcode, session, data)) as PacketHandler
    }
}
