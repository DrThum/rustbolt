use log::error;
use shipyard::View;

use crate::{
    ecs::components::movement::Movement,
    entities::position::WorldPosition,
    protocol::{
        packets::{MsgMoveTeleportAck, SmsgNewWorld, SmsgTransferPending},
        server::ServerMessage,
    },
};

use super::Player;

impl Player {
    pub fn teleport_to(&mut self, destination: &WorldPosition, force_far: bool) {
        self.set_teleport_destination(*destination);

        let Some(current_map) = self.session.current_map() else {
            error!("teleport_to: player {:?} is not on a map", self.guid());
            return;
        };
        let Some(destination_map) = self.world_context.map_manager.get_map(destination.map_key)
        else {
            error!(
                "teleport_to: destination map {} not found",
                destination.map_key
            );
            return;
        };

        let (current_position, destination_movement_info) =
            current_map
                .world()
                .run(|v_wpos: View<WorldPosition>, v_movement: View<Movement>| {
                    (
                        v_wpos[self.session.player_entity_id().unwrap()],
                        v_movement[self.session.player_entity_id().unwrap()]
                            .info(self.world_context.clone(), &destination.as_position()),
                    )
                });

        /*
         * Near teleport is a teleport without a loading screen. It's used when the destination
         * is on the same map and the world state does not need significant changes/reload.
         * Far teleport is a teleport with a loading screen. It's used when the destination is on
         * another map or the world state needs significant changes/reload.
         */
        // FIXME: For now, let's use far teleport for teleports to another map and near for the rest.
        // It's incorrect but we'll improve this later.
        let is_far_teleport = force_far || destination.map_key != current_position.map_key;

        if is_far_teleport {
            let packet = ServerMessage::new(SmsgTransferPending {
                map_id: destination.map_key.map_id,
            });

            self.session.send(&packet).unwrap();

            let packet = ServerMessage::new(SmsgNewWorld {
                map_id: destination.map_key.map_id,
                x: destination.x,
                y: destination.y,
                z: destination.z,
                o: destination.o,
            });

            self.session.send(&packet).unwrap();
            // Now the client will respond with MSG_MOVE_WORLDPORT_ACK
        } else {
            // Near teleport
            let packet = ServerMessage::new(MsgMoveTeleportAck {
                packed_guid: self.guid().as_packed(),
                unk_counter: 0,
                movement_info: destination_movement_info,
            });

            self.session.send(&packet).unwrap();
        }
    }
}
