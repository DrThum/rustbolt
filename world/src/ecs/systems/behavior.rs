use shipyard::{IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::{
        components::{
            behavior::{Action, Behavior},
            guid::Guid,
            movement::Movement,
        },
        resources::DeltaTime,
    },
    entities::{
        behaviors::{BTContext, NodeStatus},
        creature::Creature,
        position::WorldPosition,
    },
    game::map::WrappedMap,
    shared::constants::MAX_CREATURE_AGGRO_DISTANCE,
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
        Action::Aggro => action_aggro(ctx),
    }
}

fn action_aggro(ctx: &mut BTContext) -> NodeStatus {
    let my_guid = ctx.v_guid[ctx.entity_id].0;
    // TODO: Calculate the proper aggro distance depending on level
    let sessions_around =
        ctx.map
            .0
            .sessions_nearby_entity(&my_guid, MAX_CREATURE_AGGRO_DISTANCE, true, false);

    let closest = sessions_around.first(); // FIXME

    closest.map(|session| {
        let origin = ctx.v_wpos[ctx.entity_id];
        let target_entity_id = session.player_entity_id().unwrap();
        let dest = ctx.v_wpos[target_entity_id];

        let speed = ctx.vm_movement[ctx.entity_id].speed_run;
        ctx.vm_movement[ctx.entity_id].start_chasing(
            &ctx.v_guid[ctx.entity_id].0,
            &ctx.v_guid[target_entity_id].0,
            target_entity_id,
            ctx.map.0.clone(),
            &origin.vec3(),
            dest,
            speed,
            true,
        );

        return NodeStatus::Success;
    });

    NodeStatus::Failure
}
