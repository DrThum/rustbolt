use shipyard::{Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::{
        components::{
            behavior::{Action, Behavior},
            guid::Guid,
            movement::{Movement, MovementKind},
        },
        resources::DeltaTime,
    },
    entities::{
        behaviors::{BTContext, NodeStatus},
        creature::Creature,
        position::WorldPosition,
    },
    game::map::WrappedMap,
};

pub fn tick(
    dt: UniqueView<DeltaTime>,
    map: UniqueView<WrappedMap>,
    mut vm_behavior: ViewMut<Behavior>,
    mut vm_movement: ViewMut<Movement>,
    v_guid: View<Guid>,
    v_wpos: View<WorldPosition>,
    v_creature: View<Creature>,
) {
    for (entity_id, mut behavior) in (&mut vm_behavior).iter().with_id() {
        let mut context = BTContext {
            entity_id,
            dt: &dt,
            map: &map,
            vm_movement: &mut vm_movement,
            v_guid: &v_guid,
            v_wpos: &v_wpos,
            v_creature: &v_creature,
        };

        behavior.tree().tick(dt.0, &mut context, execute_action);
    }
}

fn execute_action(action: &Action, ctx: &mut BTContext) -> NodeStatus {
    match action {
        Action::WanderRandomly => action_wander_randomly(ctx),
    }
}

fn action_wander_randomly(ctx: &mut BTContext) -> NodeStatus {
    let vmm = &mut *ctx.vm_movement;
    match (vmm, ctx.v_wpos, ctx.v_creature, ctx.v_guid).get(ctx.entity_id) {
        // Choose a point to go to if we are out of combat and not currently moving
        Ok((mut mv, wpos, creature, guid)) => {
            // TODO
            // if creature.is_in_combat() {
            //    return NodeStatus::Failure
            // }

            if creature.default_movement_kind != MovementKind::Random {
                return NodeStatus::Failure;
            }

            if mv.is_moving() {
                if mv.current_movement_kind() == Some(MovementKind::Random) {
                    // Wandering in progress
                    NodeStatus::Running
                } else {
                    // Another movement is in progress, abord this one
                    NodeStatus::Failure
                }
            } else {
                // We successfully completed our wander movement
                if mv.just_finished_movement
                    && mv.previous_movement_kind() == Some(MovementKind::Random)
                {
                    NodeStatus::Success
                } else {
                    let current_pos = wpos.vec3();
                    // For the search, we need to start from:
                    //   - the creature spawn point if applicable
                    //   - the current position otherwise
                    let around = creature
                        .spawn_position
                        .map(|sp| sp.vec3())
                        .unwrap_or(current_pos);
                    let destination = ctx.map.0.get_random_point_around(
                        &around,
                        creature.wander_radius.expect(
                            "expected an existing wander radius on creature with random movement",
                        ) as f32,
                    );
                    let path = vec![destination];
                    // TODO: Select speed depending on move flags (implement in Movement)
                    let speed = mv.speed_run;
                    mv.start_random_movement(
                        &guid.0,
                        ctx.map.0.clone(),
                        &current_pos,
                        &path,
                        speed,
                        true,
                    );

                    NodeStatus::Running
                }
            }
        }
        // We're not a creature, thus not supposed to wander randomly
        _ => NodeStatus::Failure,
    }
}
