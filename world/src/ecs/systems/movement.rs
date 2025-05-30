use std::time::Instant;

use shipyard::{EntityId, Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::{
        components::{
            behavior::Behavior,
            guid::Guid,
            movement::{Movement, MovementKind},
            nearby_players::NearbyPlayers,
            powers::Powers,
            threat_list::ThreatList,
            unit::Unit,
            unwind::Unwind,
        },
        resources::DeltaTime,
    },
    entities::{
        creature::Creature,
        game_object::GameObject,
        object_guid::ObjectGuid,
        player::Player,
        position::{Position, WorldPosition},
    },
    game::{
        map::HasPlayers, movement_spline::MovementSplineState,
        packet_broadcaster::WrappedPacketBroadcaster, spatial_grid::WrappedSpatialGrid,
        terrain_manager::WrappedTerrainManager,
    },
    shared::constants::{CREATURE_LEASH_DISTANCE, MAX_CHASE_LEEWAY},
};

pub fn update_movement(
    dt: UniqueView<DeltaTime>,
    has_players: UniqueView<HasPlayers>,
    (spatial_grid, packet_broadcaster, terrain_manager): (
        UniqueView<WrappedSpatialGrid>,
        UniqueView<WrappedPacketBroadcaster>,
        UniqueView<WrappedTerrainManager>,
    ),
    v_guid: View<Guid>,
    (v_player, v_creature, v_game_object): (View<Player>, View<Creature>, View<GameObject>),
    v_powers: View<Powers>,
    (
        mut vm_unit,
        mut vm_movement,
        mut vm_wpos,
        mut vm_threat_list,
        mut vm_behavior,
        mut vm_nearby_players,
        mut vm_unwind,
    ): (
        ViewMut<Unit>,
        ViewMut<Movement>,
        ViewMut<WorldPosition>,
        ViewMut<ThreatList>,
        ViewMut<Behavior>,
        ViewMut<NearbyPlayers>,
        ViewMut<Unwind>,
    ),
) {
    if !**has_players {
        return;
    }

    let mut map_pending_updates: Vec<(&ObjectGuid, EntityId, Position)> = Vec::new();

    for (entity_id, (guid, movement, my_wpos, powers, _)) in (
        &v_guid,
        &mut vm_movement,
        &vm_wpos,
        &v_powers,
        &vm_nearby_players,
    )
        .iter()
        .with_id()
    {
        // Reset expired movements after one tick
        movement.recently_expired_movement_kinds.clear();

        if !powers.is_alive() {
            movement.reset();
            continue;
        }

        match movement.current_movement_kind() {
            MovementKind::Idle => (),
            MovementKind::Random { cooldown_end } => {
                let creature = &v_creature[entity_id];

                if movement.is_moving() {
                    let (new_position, spline_state) = movement.update(**dt);

                    let new_position = Position {
                        x: new_position.x,
                        y: new_position.y,
                        z: new_position.z,
                        o: 0., // TODO: Figure out orientation
                    };

                    map_pending_updates.push((&guid.0, entity_id, new_position));

                    if spline_state == MovementSplineState::Arrived {
                        movement.clear(false);
                    }
                } else if *cooldown_end <= Instant::now() {
                    let current_pos = my_wpos.vec3();
                    let around = creature.spawn_position.vec3();
                    let destination = terrain_manager.get_random_point_around(
                        &around,
                        creature.wander_radius.expect(
                            "expected an existing wander radius on creature with random movement",
                        ) as f32,
                    );
                    // TODO: Select speed depending on move flags (implement in Movement)
                    let speed = movement.speed_run;
                    movement.start_random_movement(
                        &guid.0,
                        (**packet_broadcaster).clone(),
                        &current_pos,
                        destination,
                        speed,
                        true,
                    );
                }
            }
            MovementKind::Path => (), // TODO
            MovementKind::Chase {
                target_guid,
                target_entity_id,
                destination,
                distance,
            } => {
                let target_entity_id = *target_entity_id;
                if let Ok(creature) = v_creature.get(entity_id) {
                    let mut should_stop_chasing = false;
                    let distance_to_home = creature.spawn_position.distance_to(my_wpos, true);
                    if distance_to_home > CREATURE_LEASH_DISTANCE {
                        should_stop_chasing = true;
                    } else if let Ok(powers) = v_powers.get(target_entity_id) {
                        if !powers.is_alive() {
                            should_stop_chasing = true;
                        }
                    }

                    if let Ok(target_position) = vm_wpos.get(target_entity_id) {
                        if destination.distance_to(target_position, true)
                            > MAX_CHASE_LEEWAY + distance
                        {
                            let target_guid = *target_guid;

                            let my_bounding_radius = vm_unit[entity_id].bounding_radius();
                            let target_bounding_radius =
                                vm_unit[target_entity_id].bounding_radius();
                            let chase_distance = my_bounding_radius + target_bounding_radius;

                            movement.clear(true);

                            let speed = movement.speed_run;
                            movement.start_chasing(
                                &guid.0,
                                &target_guid,
                                target_entity_id,
                                chase_distance,
                                (**packet_broadcaster).clone(),
                                terrain_manager.clone(),
                                my_wpos,
                                *target_position,
                                speed,
                                true,
                            );
                        } else if movement.is_moving() {
                            let (new_position, _spline_state) = movement.update(**dt);

                            let new_position = Position {
                                x: new_position.x,
                                y: new_position.y,
                                z: new_position.z,
                                o: 0., // TODO: Figure out orientation
                            };

                            map_pending_updates.push((&guid.0, entity_id, new_position));
                        }
                    } else {
                        should_stop_chasing = true;
                    }

                    if should_stop_chasing {
                        if let Ok(player) = v_player.get(target_entity_id) {
                            player.unset_in_combat_with(guid.0);
                        }

                        vm_threat_list[entity_id].remove(&target_entity_id);
                        vm_unit[entity_id].set_target(None, 0);

                        movement.clear(true);

                        let speed = movement.speed_run;
                        movement.go_to_home(
                            &guid.0,
                            (**packet_broadcaster).clone(),
                            &my_wpos.vec3(),
                            creature.spawn_position,
                            speed,
                            true,
                        );
                        continue;
                    }
                }
            }
            MovementKind::ReturnHome => {
                let (new_position, spline_state) = movement.update(**dt);

                let new_position = Position {
                    x: new_position.x,
                    y: new_position.y,
                    z: new_position.z,
                    o: 0., // TODO: Figure out orientation
                };

                map_pending_updates.push((&guid.0, entity_id, new_position));

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
    for (guid, entity_id, pos) in map_pending_updates {
        spatial_grid.update_entity_position(
            guid,
            entity_id,
            None, // FIXME: Must be defined if it's a server-controlled player (feared for example)
            &pos,
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
    }
}
