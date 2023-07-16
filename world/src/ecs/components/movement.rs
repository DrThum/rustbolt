use std::sync::Arc;

use shipyard::Component;

use crate::{
    entities::{position::Position, update::MovementUpdateData},
    game::world_context::WorldContext,
    protocol::{
        packets::{SmsgMoveSetCanFly, SmsgMoveUnsetCanFly},
        server::ServerMessage,
    },
    session::world_session::WorldSession,
};

#[derive(Component)]
pub struct Movement {
    pub flags: u32, // TODO: enum MovementFlag
    pub pitch: Option<f32>,
    pub fall_time: u32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub speed_run_backward: f32,
    pub speed_swim: f32,
    pub speed_swim_backward: f32,
    pub speed_flight: f32,
    pub speed_flight_backward: f32,
    pub speed_turn: f32,
}

impl Movement {
    pub fn build_update(
        &self,
        world_context: Arc<WorldContext>,
        position: &Position,
    ) -> MovementUpdateData {
        MovementUpdateData {
            movement_flags: self.flags,
            movement_flags2: 0, // Always 0 in 2.4.3
            timestamp: world_context.game_time().as_millis() as u32, // Will overflow every 49.7 days
            position: *position,
            pitch: self.pitch,
            fall_time: self.fall_time,
            speed_walk: self.speed_walk,
            speed_run: self.speed_run,
            speed_run_backward: self.speed_run_backward,
            speed_swim: self.speed_swim,
            speed_swim_backward: self.speed_swim_backward,
            speed_flight: self.speed_flight,
            speed_flight_backward: self.speed_flight_backward,
            speed_turn: self.speed_turn,
        }
    }

    // TODO: Improper implementation:
    // - movement packets should share a common implementation
    // - correct workflow is SMSG_MOVE_XXX -> CMSG_MOVE_XXX_ACK -> MSG_MOVE_XXX to send to the
    // players around
    pub fn set_flying(&mut self, flying: bool, session: Arc<WorldSession>) {
        if flying {
            self.flags &= 0x03000000; // CanFly & PlayerFlying
            self.pitch = Some(0.);

            let packet =
                ServerMessage::new(SmsgMoveSetCanFly::build(&session.player_guid().unwrap()));
            session.send(&packet).unwrap();
        } else {
            self.flags ^= 0x03000000;
            self.pitch = None;

            let packet =
                ServerMessage::new(SmsgMoveUnsetCanFly::build(&session.player_guid().unwrap()));
            session.send(&packet).unwrap();
        }
    }
}
