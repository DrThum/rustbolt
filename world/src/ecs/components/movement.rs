use std::{sync::Arc, time::Duration};

use enumflags2::{BitFlag, BitFlags};
use shared::models::terrain_info::Vector3;
use shipyard::Component;

use crate::{
    entities::{
        position::Position,
        update::{CurrentMovementData, MovementUpdateData},
    },
    game::{
        movement_spline::{MovementSpline, MovementSplineState},
        world_context::WorldContext,
    },
    protocol::{
        packets::{SmsgMoveSetCanFly, SmsgMoveUnsetCanFly},
        server::ServerMessage,
    },
    session::world_session::WorldSession,
    shared::constants::MovementFlag,
};

#[derive(Component)]
pub struct Movement {
    pub flags: BitFlags<MovementFlag>,
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
    spline: MovementSpline,
}

impl Movement {
    pub fn new() -> Self {
        Self {
            flags: MovementFlag::empty(),
            pitch: None,
            fall_time: 0,
            speed_walk: 2.5,
            speed_run: 7.0,
            speed_run_backward: 4.5,
            speed_swim: 4.722222,
            speed_swim_backward: 2.5,
            speed_flight: 70.0,
            speed_flight_backward: 4.5,
            speed_turn: 3.141594,
            spline: MovementSpline::new(),
        }
    }

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
            current_movement: CurrentMovementData::build(self.flags, &self.spline),
        }
    }

    // TODO: Improper implementation:
    // - movement packets should share a common implementation
    // - correct workflow is SMSG_MOVE_XXX -> CMSG_MOVE_XXX_ACK -> MSG_MOVE_XXX to send to the
    // players around
    pub fn set_flying(&mut self, flying: bool, session: Arc<WorldSession>) {
        if flying {
            self.flags
                .insert(MovementFlag::CanFly | MovementFlag::PlayerFlying);
            self.pitch = Some(0.);

            let packet =
                ServerMessage::new(SmsgMoveSetCanFly::build(&session.player_guid().unwrap()));
            session.send(&packet).unwrap();
        } else {
            self.flags
                .remove(MovementFlag::CanFly | MovementFlag::PlayerFlying);
            self.pitch = None;

            let packet =
                ServerMessage::new(SmsgMoveUnsetCanFly::build(&session.player_guid().unwrap()));
            session.send(&packet).unwrap();
        }
    }

    pub fn start_movement(
        &mut self,
        starting_position: &Vector3,
        path: &Vec<Vector3>,
        velocity: f32,
        linear: bool,
    ) -> Duration {
        self.flags
            .insert(MovementFlag::SplineEnabled | MovementFlag::Forward);
        self.spline.init(starting_position, path, velocity, linear)
    }

    pub fn is_moving(&self) -> bool {
        self.spline.state() == MovementSplineState::Moving
    }

    pub fn spline(&self) -> &MovementSpline {
        &self.spline
    }

    pub fn update(&mut self, dt: Duration) -> (Vector3, MovementSplineState) {
        self.spline.update(dt)
    }

    pub fn reset_spline(&mut self) {
        self.spline.reset();

        self.flags
            .remove(MovementFlag::SplineEnabled | MovementFlag::Forward);
    }
}
