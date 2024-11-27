use std::sync::Arc;

use shipyard::{View, ViewMut};

use crate::{
    ecs::components::{
        behavior::Behavior, guid::Guid, movement::Movement, nearby_players::NearbyPlayers,
        unwind::Unwind,
    }, entities::{
        creature::Creature, game_object::GameObject, player::Player, position::WorldPosition,
    }, game::world_context::WorldContext, protocol::{client::ClientMessage, opcodes::Opcode, packets::{MovementInfo, MsgMoveTeleportAckFromClient}}, session::{
        opcode_handler::{OpcodeHandler, PacketHandler},
        world_session::WorldSession,
    }
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
                 v_game_object: View<GameObject>,
                 v_guid: View<Guid>,
                 mut vm_wpos: ViewMut<WorldPosition>,
                 mut vm_behavior: ViewMut<Behavior>,
                 mut vm_nearby_players: ViewMut<NearbyPlayers>,
                 mut vm_unwind: ViewMut<Unwind>| {
                    map.update_entity_position(
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

        Box::new(move |session, _, data| handle_movement(opcode, session, data)) as PacketHandler
    }

    // Used for near teleports
    pub fn handle_msg_move_teleport_ack(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let _msg: MsgMoveTeleportAckFromClient = ClientMessage::read_as(data).unwrap();

        let map = session.current_map().unwrap();
        let player_guid = session.player_guid().unwrap();

        // Register new position
        map.world().run(
            |v_movement: View<Movement>,
                v_creature: View<Creature>,
                v_game_object: View<GameObject>,
                v_guid: View<Guid>,
                mut vm_player: ViewMut<Player>,
                mut vm_wpos: ViewMut<WorldPosition>,
                mut vm_behavior: ViewMut<Behavior>,
                mut vm_nearby_players: ViewMut<NearbyPlayers>,
                mut vm_unwind: ViewMut<Unwind>| {
                let player_entity_id = session.player_entity_id().unwrap();

                let world_pos = vm_player[player_entity_id].take_teleport_destination().expect("player was teleported with no destination");

                map.update_entity_position(
                    &player_guid,
                    player_entity_id,
                    Some(session.clone()),
                    &world_pos.as_position(),
                    &v_movement,
                    &vm_player.as_view(),
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
    }
}
