use std::time::Instant;

use shipyard::{Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::{
        components::{
            guid::Guid,
            movement::{Movement, MovementKind},
        },
        resources::DeltaTime,
    },
    entities::{
        creature::Creature,
        object_guid::ObjectGuid,
        player::Player,
        position::{Position, WorldPosition},
    },
    game::{map::WrappedMap, movement_spline::MovementSplineState},
    shared::constants::{CREATURE_LEASH_DISTANCE, MAX_CHASE_LEEWAY},
};

pub fn update_movement(
    dt: UniqueView<DeltaTime>,
    map: UniqueView<WrappedMap>,
    v_guid: View<Guid>,
    v_player: View<Player>,
    v_creature: View<Creature>,
    mut vm_movement: ViewMut<Movement>,
    mut vm_wpos: ViewMut<WorldPosition>,
) {
    let mut map_pending_updates: Vec<(&ObjectGuid, Position)> = Vec::new();

    for (entity_id, (guid, mut movement, my_wpos)) in
        (&v_guid, &mut vm_movement, &vm_wpos).iter().with_id()
    {
        // Reset expired movements after one tick
        movement.recently_expired_movement_kinds.clear();

        match movement.current_movement_kind() {
            MovementKind::Idle => (),
            MovementKind::Random { cooldown_end } => {
                let creature = &v_creature[entity_id];

                if movement.is_moving() {
                    let (new_position, spline_state) = movement.update(dt.0);

                    let new_position = Position {
                        x: new_position.x,
                        y: new_position.y,
                        z: new_position.z,
                        o: 0., // TODO: Figure out orientation
                    };

                    map_pending_updates.push((&guid.0, new_position));

                    if spline_state == MovementSplineState::Arrived {
                        movement.clear(false);
                    }
                } else {
                    if *cooldown_end <= Instant::now() {
                        let current_pos = my_wpos.vec3();
                        // For the search, we need to start from:
                        //   - the creature spawn point if applicable
                        //   - the current position otherwise
                        let around = creature
                            .spawn_position
                            .map(|sp| sp.vec3())
                            .unwrap_or(current_pos);
                        let destination = map.0.get_random_point_around(
                        &around,
                        creature.wander_radius.expect(
                            "expected an existing wander radius on creature with random movement",
                        ) as f32,
                    );
                        let path = vec![destination];
                        // TODO: Select speed depending on move flags (implement in Movement)
                        let speed = movement.speed_run;
                        movement.start_random_movement(
                            &guid.0,
                            map.0.clone(),
                            &current_pos,
                            &path,
                            speed,
                            true,
                        );
                    }
                }
            }
            MovementKind::Path => (), // TODO
            MovementKind::Chase {
                target_guid,
                target_entity_id,
                destination,
            } => {
                if let Ok(creature) = v_creature.get(entity_id) {
                    if let Some(spawn_pos) = creature.spawn_position {
                        let distance_to_home = spawn_pos.distance_to(my_wpos, true);
                        println!("distance to home {distance_to_home}");
                        if distance_to_home > CREATURE_LEASH_DISTANCE {
                            movement.clear(true);
                            let speed = movement.speed_run;
                            movement.go_to_home(
                                &guid.0,
                                map.0.clone(),
                                &my_wpos.vec3(),
                                spawn_pos,
                                speed,
                                true,
                            );
                            continue;
                        }
                    }
                }

                let target_position = vm_wpos[*target_entity_id];
                if destination.distance_to(&target_position, true) > MAX_CHASE_LEEWAY {
                    let target_guid = target_guid.clone();
                    let target_entity_id = *target_entity_id;

                    movement.clear(true);

                    let speed = movement.speed_run;
                    movement.start_chasing(
                        &guid.0,
                        &target_guid,
                        target_entity_id,
                        map.0.clone(),
                        &my_wpos.vec3(),
                        target_position,
                        speed,
                        true,
                    );
                } else if movement.is_moving() {
                    let (new_position, _spline_state) = movement.update(dt.0);

                    let new_position = Position {
                        x: new_position.x,
                        y: new_position.y,
                        z: new_position.z,
                        o: 0., // TODO: Figure out orientation
                    };

                    map_pending_updates.push((&guid.0, new_position));
                }
            }
            MovementKind::ReturnHome => {
                let (new_position, spline_state) = movement.update(dt.0);

                let new_position = Position {
                    x: new_position.x,
                    y: new_position.y,
                    z: new_position.z,
                    o: 0., // TODO: Figure out orientation
                };

                map_pending_updates.push((&guid.0, new_position));

                if spline_state == MovementSplineState::Arrived {
                    movement.clear(true);
                }
            }
            MovementKind::PlayerControlled => (),
        }
    }

    // Update the map out of the loop because we need to borrow View<Movement> and the loop already
    // borrows ViewMut<Movement>
    let v_movement = vm_movement.as_view();
    for (guid, pos) in map_pending_updates {
        map.0.update_entity_position(
            guid,
            None, // FIXME: Must be defined if it's a server-controlled player (feared for example)
            &pos,
            &v_movement,
            &v_player,
            &v_creature,
            &mut vm_wpos,
        );
    }
}
