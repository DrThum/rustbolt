use std::sync::Arc;

use chrono::{Datelike, Timelike};
use log::error;
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
    protocol::{
        client::ClientMessage,
        opcodes::Opcode,
        packets::{
            MovementInfo, MsgMoveTeleportAckFromClient, SmsgBindpointupdate, SmsgInitWorldStates,
            SmsgLoginSetTimeSpeed, SmsgSetRestStart, SmsgTutorialFlags,
        },
        server::ServerMessage,
    },
    session::{
        opcode_handler::{OpcodeHandler, PacketHandler, PacketHandlerArgs},
        world_session::{WorldSession, WorldSessionState},
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

    // TODO: deduplicate with handle_cmsg_player_login
    pub fn handle_msg_move_worldport_ack(
        PacketHandlerArgs {
            session,
            world_context,
            ..
        }: PacketHandlerArgs,
    ) {
        let smsg_set_rest_start = ServerMessage::new(SmsgSetRestStart { rest_start: 0 });

        session.send(&smsg_set_rest_start).unwrap();

        // TODO
        let smsg_bindpointupdate = ServerMessage::new(SmsgBindpointupdate {
            homebind_x: -8953.95,
            homebind_y: 521.019,
            homebind_z: 96.5399,
            homebind_map_id: 0,
            homebind_area_id: 85,
        });

        session.send(&smsg_bindpointupdate).unwrap();

        let smsg_tutorial_flags = ServerMessage::new(SmsgTutorialFlags {
            tutorial_data0: 0, // FIXME: 0xFFFFFFFF to disable tutorials
            tutorial_data1: 0,
            tutorial_data2: 0,
            tutorial_data3: 0,
            tutorial_data4: 0,
            tutorial_data5: 0,
            tutorial_data6: 0,
            tutorial_data7: 0,
        });

        session.send(&smsg_tutorial_flags).unwrap();

        // The client expects a specific format which is not unix timestamp
        // See secsToTimeBitFields in MaNGOS
        let timestamp: u32 = {
            let now = chrono::Local::now();

            let year = now.year() as u32;
            let month = now.month();
            let month_day = now.day() - 1;
            let weekday = now.weekday().number_from_sunday();
            let hour = now.hour();
            let minutes = now.minute();

            (year << 24)
                | (month << 20)
                | (month_day << 14)
                | (weekday << 11)
                | (hour << 6)
                | minutes
        };

        let smsg_login_set_time_speed = ServerMessage::new(SmsgLoginSetTimeSpeed {
            timestamp,
            game_speed: 0.01666667,
        });

        session.send(&smsg_login_set_time_speed).unwrap();

        {
            let mut session_state = session.state.write();
            *session_state = WorldSessionState::InWorld;
        }

        let Some(teleport_position) = session.current_map().unwrap().world().run(
            |mut vm_player: ViewMut<Player>| {
                vm_player[session.player_entity_id().unwrap()].take_teleport_destination()
            },
        ) else {
            error!("received MSG_MOVE_WORLDPORT_ACK with no destination stored on Player");
            return;
        };

        if let Some(destination_map) = world_context.map_manager.get_map(teleport_position.map_key)
        {
            session.set_map(destination_map.clone());
            destination_map.transfer_player_from_other_map(session.clone());
        }

        // FIXME: hardcoded position
        let smsg_init_world_states = ServerMessage::new(SmsgInitWorldStates {
            map_id: 0,
            zone_id: 85,
            area_id: 154, // Deathknell
            block_count: 0,
        });

        session.send(&smsg_init_world_states).unwrap();

        WorldSession::reset_time_sync(session, world_context);
    }
}
