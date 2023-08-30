use std::sync::Arc;

use shipyard::{Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::{
        components::{
            behavior::{Action, Behavior},
            guid::Guid,
            health::Health,
            melee::Melee,
            movement::{Movement, MovementKind},
            spell_cast::SpellCast,
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
    shared::constants::{CREATURE_AGGRO_DISTANCE_MAX, MAX_CHASE_LEEWAY},
};

pub fn tick(
    dt: UniqueView<DeltaTime>,
    map: UniqueView<WrappedMap>,
    (mut vm_behavior, mut vm_movement): (ViewMut<Behavior>, ViewMut<Movement>),
    mut vm_unit: ViewMut<Unit>,
    mut vm_threat_list: ViewMut<ThreatList>,
    mut vm_melee: ViewMut<Melee>,
    v_guid: View<Guid>,
    v_wpos: View<WorldPosition>,
    v_creature: View<Creature>,
    (v_player, mut vm_health, v_spell): (View<Player>, ViewMut<Health>, View<SpellCast>),
) {
    for (entity_id, mut behavior) in (&mut vm_behavior).iter().with_id() {
        let mut context = BTContext {
            entity_id,
            dt: &dt,
            map: &map,
            vm_movement: &mut vm_movement,
            vm_unit: &mut vm_unit,
            vm_threat_list: &mut vm_threat_list,
            vm_health: &mut vm_health,
            vm_melee: &mut vm_melee,
            v_guid: &v_guid,
            v_wpos: &v_wpos,
            v_creature: &v_creature,
            v_player: &v_player,
            v_spell: &v_spell,
        };

        behavior.tree().tick(dt.0, &mut context, execute_action);
    }
}

fn execute_action(action: &Action, ctx: &mut BTContext) -> NodeStatus {
    match action {
        Action::Aggro => action_aggro(ctx),
        Action::AttackInMelee => action_attack_in_melee(ctx),
        Action::ChaseTarget => action_chase_target(ctx),
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
                    let player_health = &ctx.vm_health[player_entity_id];
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
        let target_entity_id = session.player_entity_id().unwrap();

        if let Ok(player) = ctx.v_player.get(target_entity_id) {
            player.set_in_combat_with(my_guid);
            threat_list.modify_threat(target_entity_id, 0.);
            ctx.vm_unit[ctx.entity_id].set_target(Some(target_entity_id), player.guid().raw());
        }

        return NodeStatus::Success;
    });

    NodeStatus::Failure
}

fn action_attack_in_melee(ctx: &mut BTContext) -> NodeStatus {
    let my_id = ctx.entity_id;
    let unit = &mut ctx.vm_unit[my_id];

    if let Some(target_id) = unit.target() {
        if let Ok(target_guid) = ctx.v_guid.get(target_id).map(|g| g.0) {
            match Melee::execute_attack(
                my_id,
                target_id,
                target_guid,
                ctx.map.0.clone(),
                ctx.v_guid,
                ctx.v_wpos,
                ctx.v_spell,
                ctx.vm_health,
                ctx.vm_melee,
                ctx.vm_threat_list,
            ) {
                Ok(_) => return NodeStatus::Success,
                Err(_) => return NodeStatus::Failure,
            }
        } else {
            unit.set_target(None, 0);
        }
    }

    NodeStatus::Failure
}

fn action_chase_target(ctx: &mut BTContext) -> NodeStatus {
    let unit_me = &ctx.vm_unit[ctx.entity_id];
    if let Some(target_entity_id) = unit_me.target() {
        let my_bounding_radius = ctx.vm_unit[ctx.entity_id].bounding_radius();
        let target_bounding_radius = ctx.vm_unit[target_entity_id].bounding_radius();
        let chase_distance = my_bounding_radius + target_bounding_radius;
        let my_current_position = ctx.v_wpos[ctx.entity_id];
        let target_position = ctx.v_wpos[target_entity_id];

        // Change movement if
        // - already chasing but chasing the wrong target
        // - not chasing and the target is too far away from us
        let (should_init_movement, should_clear) =
            match ctx.vm_movement[ctx.entity_id].current_movement_kind() {
                MovementKind::Chase {
                    target_entity_id: chasing_entity_id,
                    ..
                } => (*chasing_entity_id != target_entity_id, true),
                _ => (
                    my_current_position.distance_to(&target_position, true) > MAX_CHASE_LEEWAY,
                    false,
                ),
            };

        if should_init_movement {
            if should_clear {
                ctx.vm_movement[ctx.entity_id].clear(true);
            }

            let speed = ctx.vm_movement[ctx.entity_id].speed_run;
            ctx.vm_movement[ctx.entity_id].start_chasing(
                &ctx.v_guid[ctx.entity_id].0,
                &ctx.v_guid[target_entity_id].0,
                target_entity_id,
                chase_distance,
                ctx.map.0.clone(),
                &my_current_position,
                target_position,
                speed,
                true,
            );

            return NodeStatus::Success;
        }
    }

    NodeStatus::Failure
}
