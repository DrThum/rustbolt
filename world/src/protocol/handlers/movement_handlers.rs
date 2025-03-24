use std::sync::Arc;

use shipyard::{UniqueView, View, ViewMut};

use crate::{
    ecs::components::{
        behavior::Behavior, guid::Guid, movement::Movement, nearby_players::NearbyPlayers,
        unwind::Unwind,
    },
    entities::{
        creature::Creature, game_object::GameObject, player::Player, position::WorldPosition,
    },
    game::spatial_grid::WrappedSpatialGrid,
    protocol::{client::ClientMessage, opcodes::Opcode, packets::MovementInfo},
    session::{
        opcode_handler::{OpcodeHandler, PacketHandler, PacketHandlerArgs},
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
                |spatial_grid: UniqueView<WrappedSpatialGrid>,
                 v_movement: View<Movement>,
                 v_player: View<Player>,
                 v_creature: View<Creature>,
                 v_game_object: View<GameObject>,
                 v_guid: View<Guid>,
                 mut vm_wpos: ViewMut<WorldPosition>,
                 mut vm_behavior: ViewMut<Behavior>,
                 mut vm_nearby_players: ViewMut<NearbyPlayers>,
                 mut vm_unwind: ViewMut<Unwind>| {
                    spatial_grid.update_entity_position(
                        &player_guid,
                        session.player_entity_id().unwrap(),
                        Some(session.clone()),
                        &movement_info.position,
                        &v_movement,
                        &v_player,
                        &v_creature,
                        &v_game_object,
                        &v_guid,
                        &mut vm_wpos,
                        &mut vm_behavior,
                        &mut vm_nearby_players,
                        &mut vm_unwind,
                    );
                },
            );

            // Broadcast to nearby players
            map.broadcast_movement(&player_guid, opcode, &movement_info);
        }

        Box::new(
            move |PacketHandlerArgs { session, data, .. }: PacketHandlerArgs| {
                handle_movement(opcode, session, data)
            },
        ) as PacketHandler
    }
}
