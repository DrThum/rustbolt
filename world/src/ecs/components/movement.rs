use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use enumflags2::{BitFlag, BitFlags};
use rand::Rng;
use rusqlite::types::{FromSql, FromSqlError};
use shared::models::terrain_info::Vector3;
use shipyard::{Component, EntityId};

use crate::{
    entities::{
        object_guid::ObjectGuid,
        position::{Position, WorldPosition},
        update::{CurrentMovementData, MovementUpdateData},
    },
    game::{
        map::Map,
        movement_spline::{MovementSpline, MovementSplineState},
        world_context::WorldContext,
    },
    protocol::{
        packets::{SmsgMonsterMove, SmsgMoveSetCanFly, SmsgMoveUnsetCanFly},
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
    // Movements that expired during the latest tick
    pub recently_expired_movement_kinds: Vec<MovementKind>,
    // Acts like a stack, the top of the stack is at the end of the Vec
    current_movement_kinds: Vec<MovementKind>,
}

impl Movement {
    pub fn new(default_movement_kind: MovementKind) -> Self {
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
            recently_expired_movement_kinds: Vec::new(),
            current_movement_kinds: vec![default_movement_kind],
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
        mover_guid: &ObjectGuid,
        map: Arc<Map>,
        starting_position: &Vector3,
        path: &Vec<Vector3>,
        velocity: f32,
        linear: bool,
    ) -> Duration {
        self.flags
            .insert(MovementFlag::SplineEnabled | MovementFlag::Forward);
        let spline_duration = self.spline.init(starting_position, path, velocity, linear);

        let packet = ServerMessage::new(SmsgMonsterMove::build(
            mover_guid,
            starting_position,
            path.to_vec(),
            0,
            0,
            self.spline.spline_flags(),
            spline_duration.as_millis() as u32,
        ));

        map.broadcast_packet(mover_guid, &packet, None, true);
        spline_duration
    }

    pub fn start_random_movement(
        &mut self,
        mover_guid: &ObjectGuid,
        map: Arc<Map>,
        starting_position: &Vector3,
        path: &Vec<Vector3>,
        velocity: f32,
        linear: bool,
    ) -> Duration {
        assert!(
            self.current_movement_kinds.len() == 1
                && self.current_movement_kinds.first().unwrap().is_random(),
            "random movement should always be on the bottom of the stack and never expire"
        );

        let duration =
            self.start_movement(mover_guid, map, starting_position, path, velocity, linear);

        // Calculate the cooldown according to the skip chance
        let mut rng = rand::thread_rng();
        let mut new_cooldown_end = Instant::now() + duration;
        if rng.gen_range(0.0..1.0) > WANDER_COOLDOWN_SKIP_CHANCE {
            let extra_cooldown = rng.gen_range(WANDER_COOLDOWN_MIN..WANDER_COOLDOWN_MAX);
            new_cooldown_end += extra_cooldown;
        }

        match self.current_movement_kind_mut() {
            MovementKind::Random { cooldown_end } => *cooldown_end = new_cooldown_end,
            _ => panic!("expected random movement kind"),
        }

        duration
    }

    pub fn start_chasing(
        &mut self,
        mover_guid: &ObjectGuid,
        target_guid: &ObjectGuid,
        target_entity_id: EntityId,
        map: Arc<Map>,
        starting_position: &Vector3,
        destination: WorldPosition,
        velocity: f32,
        linear: bool,
    ) -> Duration {
        self.current_movement_kinds.push(MovementKind::Chase {
            target_guid: *target_guid,
            target_entity_id,
            destination,
        });
        self.start_movement(
            mover_guid,
            map,
            starting_position,
            &vec![destination.vec3()],
            velocity,
            linear,
        )
    }

    pub fn go_to_home(
        &mut self,
        mover_guid: &ObjectGuid,
        map: Arc<Map>,
        starting_position: &Vector3,
        destination: WorldPosition,
        velocity: f32,
        linear: bool,
    ) -> Duration {
        self.current_movement_kinds
            .push(MovementKind::ReturnHome);
        self.start_movement(
            mover_guid,
            map,
            starting_position,
            &vec![destination.vec3()],
            velocity,
            linear,
        )
    }

    pub fn is_moving(&self) -> bool {
        self.spline.state() == MovementSplineState::Moving
    }

    pub fn previous_movement_kind(&self) -> &Vec<MovementKind> {
        &self.recently_expired_movement_kinds
    }

    pub fn current_movement_kind(&self) -> &MovementKind {
        self.current_movement_kinds
            .last()
            .expect("unexpected empty movement kinds stack")
    }

    pub fn current_movement_kind_mut(&mut self) -> &mut MovementKind {
        self.current_movement_kinds
            .last_mut()
            .expect("unexpected empty movement kinds stack")
    }

    pub fn spline(&self) -> &MovementSpline {
        &self.spline
    }

    pub fn update(&mut self, dt: Duration) -> (Vector3, MovementSplineState) {
        self.spline.update(dt)
    }

    pub fn clear(&mut self, should_expire: bool) {
        if should_expire {
            let expired = self
                .current_movement_kinds
                .pop()
                .expect("unexpected empty movement kinds stack");
            self.recently_expired_movement_kinds.push(expired);
        }
        self.spline.reset();

        self.flags
            .remove(MovementFlag::SplineEnabled | MovementFlag::Forward);
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum MovementKind {
    Idle,
    Random {
        cooldown_end: Instant,
    }, // Randomly moving around
    Path, // aka Waypoint
    Chase {
        target_guid: ObjectGuid,
        target_entity_id: EntityId,
        destination: WorldPosition,
    },
    PlayerControlled,
    ReturnHome, // aka Evade
                   // Feared,
                   // ...
}

impl MovementKind {
    pub fn is_random(&self) -> bool {
        match self {
            MovementKind::Random { cooldown_end: _ } => true,
            _ => false,
        }
    }
}

impl FromSql for MovementKind {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_i64() {
            Ok(0) => Ok(MovementKind::Idle),
            Ok(1) => Ok(MovementKind::Random {
                cooldown_end: Instant::now(),
            }),
            Ok(2) => Ok(MovementKind::Path),
            Ok(_) => Err(FromSqlError::Other(
                "database movement type can only be Idle, Random or Path".into(),
            )),
            Err(_) => Err(FromSqlError::Other("invalid movement type".into())),
        }
    }
}

pub const WANDER_COOLDOWN_MIN: Duration = Duration::from_secs(3);
pub const WANDER_COOLDOWN_MAX: Duration = Duration::from_secs(10);
pub const WANDER_COOLDOWN_SKIP_CHANCE: f32 = 0.3;
