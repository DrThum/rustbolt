use std::sync::Arc;

use log::{error, warn};
use shipyard::{Get, UniqueView, View, ViewMut};

use crate::{
    ecs::components::{
        behavior::Behavior, guid::Guid, movement::Movement, nearby_players::NearbyPlayers,
        unwind::Unwind,
    },
    entities::{
        creature::Creature, game_object::GameObject, player::Player, position::WorldPosition,
    },
    game::spatial_grid::WrappedSpatialGrid,
    protocol::{
        client::ClientMessage,
        opcodes::Opcode,
        packets::{MovementInfo, MsgMoveTeleportAckFromClient},
    },
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

    // Used for near teleports
    pub fn handle_msg_move_teleport_ack(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let _msg: MsgMoveTeleportAckFromClient = ClientMessage::read_as(data).unwrap();
        let map = session.current_map().unwrap();
        let player_guid = session.player_guid().unwrap();

        // Register new position
        map.world().run(
            |spatial_grid: UniqueView<WrappedSpatialGrid>,
             v_movement: View<Movement>,
             v_creature: View<Creature>,
             v_game_object: View<GameObject>,
             v_guid: View<Guid>,
             mut vm_player: ViewMut<Player>,
             mut vm_wpos: ViewMut<WorldPosition>,
             mut vm_behavior: ViewMut<Behavior>,
             mut vm_nearby_players: ViewMut<NearbyPlayers>,
             mut vm_unwind: ViewMut<Unwind>| {
                let player_entity_id = session.player_entity_id().unwrap();

                let world_pos = vm_player[player_entity_id]
                    .take_teleport_destination()
                    .expect("player was teleported with no destination");

                spatial_grid.update_entity_position(
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

    pub fn handle_msg_move_worldport_ack(
        PacketHandlerArgs {
            session,
            world_context,
            ..
        }: PacketHandlerArgs,
    ) {
        let Some(map) = session.current_map() else {
            error!("handle_msg_move_worldport_ack: session has no map");
            return;
        };
        let Some(player_entity_id) = session.player_entity_id() else {
            error!("handle_msg_move_worldport_ack: session has no player EntityId");
            return;
        };

        map.world().run(|v_player: View<Player>| {
            if let Ok(player) = v_player.get(player_entity_id) {
                session.send_initial_packets_before_add_to_map(player.bindpoint());
            }
        });

        let Some(teleport_position) = session.current_map().unwrap().world().run(
            |mut vm_player: ViewMut<Player>, mut vm_wpos: ViewMut<WorldPosition>| {
                let teleport_position = vm_player[player_entity_id].take_teleport_destination();
                match teleport_position {
                    Some(pos) => vm_wpos[player_entity_id].update(&pos),
                    None => warn!("handle_msg_move_worldport_ack: teleported player has no teleport destination"),
                }

                teleport_position
            },
        ) else {
            error!("received MSG_MOVE_WORLDPORT_ACK with no destination stored on Player");
            return;
        };

        if let Some(destination_map) = world_context.map_manager.get_map(teleport_position.map_key)
        {
            destination_map.transfer_player_from_other_map(session.clone());
            session.send_initial_packets_after_add_to_map(world_context.clone());
        }
    }
}
