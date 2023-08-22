use std::sync::Arc;

use shipyard::{Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::{
        components::{
            behavior::{Action, Behavior},
            guid::Guid,
            health::Health,
            movement::Movement,
            threat_list::ThreatList,
            unit::Unit,
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
    mut vm_unit: ViewMut<Unit>,
    mut vm_threat_list: ViewMut<ThreatList>,
    v_guid: View<Guid>,
    v_wpos: View<WorldPosition>,
    v_creature: View<Creature>,
    (v_player, v_health): (View<Player>, View<Health>),
) {
    for (entity_id, mut behavior) in (&mut vm_behavior).iter().with_id() {
        let mut context = BTContext {
            entity_id,
            dt: &dt,
            map: &map,
            vm_movement: &mut vm_movement,
            vm_unit: &mut vm_unit,
            vm_threat_list: &mut vm_threat_list,
            v_guid: &v_guid,
            v_wpos: &v_wpos,
            v_creature: &v_creature,
            v_player: &v_player,
            v_health: &v_health,
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
    let threat_list = &mut ctx.vm_threat_list[ctx.entity_id];
    let unit_me = &ctx.vm_unit[ctx.entity_id];

    let sessions_around: Vec<Arc<WorldSession>> = ctx
        .map
        .0
        .sessions_nearby_entity(&my_guid, CREATURE_AGGRO_DISTANCE_MAX, true, false)
        .into_iter()
        .filter(|session| {
            session
                .player_entity_id()
                .map(|player_entity_id| {
                    let player = &ctx.v_player[player_entity_id];
                    let player_position = ctx.v_wpos[player_entity_id];
                    let player_health = &ctx.v_health[player_entity_id];
                    let target_unit = &ctx.vm_unit[player_entity_id];

                    player_health.is_alive()
                        && unit_me.is_hostile_to(&target_unit)
                        && creature_position.distance_to(&player_position, true)
                            <= creature.aggro_distance(player.level())
                })
                .unwrap_or(false)
        })
        .collect();

    let closest = sessions_around.first(); // FIXME

    closest.map(|session| {
        let origin = ctx.v_wpos[ctx.entity_id];
        let target_entity_id = session.player_entity_id().unwrap();
        let dest = ctx.v_wpos[target_entity_id];

        if let Ok(player) = ctx.v_player.get(target_entity_id) {
            player.set_in_combat_with(my_guid);
            threat_list.modify_threat(player.guid(), 0.);
            ctx.vm_unit[ctx.entity_id].set_target(Some(target_entity_id), player.guid().raw());
        }

        let my_bounding_radius = ctx.vm_unit[ctx.entity_id].bounding_radius();
        let target_bounding_radius = ctx.vm_unit[target_entity_id].bounding_radius();
        let chase_distance = my_bounding_radius + target_bounding_radius;

        let speed = ctx.vm_movement[ctx.entity_id].speed_run;
        ctx.vm_movement[ctx.entity_id].start_chasing(
            &ctx.v_guid[ctx.entity_id].0,
            &ctx.v_guid[target_entity_id].0,
            target_entity_id,
            chase_distance,
            ctx.map.0.clone(),
            &origin,
            dest,
            speed,
            true,
        );

        return NodeStatus::Success;
    });

    NodeStatus::Failure
}
