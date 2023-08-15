use std::sync::Arc;

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
        player::Player,
        position::WorldPosition,
    },
    game::map::WrappedMap,
    session::world_session::WorldSession,
    shared::constants::CREATURE_AGGRO_DISTANCE_MAX,
};

pub fn tick(
    dt: UniqueView<DeltaTime>,
    map: UniqueView<WrappedMap>,
    mut vm_behavior: ViewMut<Behavior>,
    mut vm_movement: ViewMut<Movement>,
    v_guid: View<Guid>,
    v_wpos: View<WorldPosition>,
    v_creature: View<Creature>,
    v_player: View<Player>,
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
            v_player: &v_player,
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
    let creature = &ctx.v_creature[ctx.entity_id];
    let creature_position = &ctx.v_wpos[ctx.entity_id];

    let sessions_around: Vec<Arc<WorldSession>> = ctx
        .map
        .0
        .sessions_nearby_entity(&my_guid, CREATURE_AGGRO_DISTANCE_MAX, true, false)
        .into_iter()
        .filter(|session| {
            let player_entity_id = session.player_entity_id().unwrap();
            let player = &ctx.v_player[player_entity_id];
            let player_position = ctx.v_wpos[player_entity_id];

            creature_position.distance_to(&player_position, true)
                <= creature.aggro_distance(player.level())
        })
        .collect();

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
