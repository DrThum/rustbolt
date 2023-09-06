use shipyard::{EntityId, Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

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
    game::{map::WrappedMap, world_context::WrappedWorldContext},
    shared::constants::MAX_CHASE_LEEWAY,
};

pub fn tick(
    dt: UniqueView<DeltaTime>,
    (map, world_context): (UniqueView<WrappedMap>, UniqueView<WrappedWorldContext>),
    (mut vm_behavior, mut vm_movement): (ViewMut<Behavior>, ViewMut<Movement>),
    mut vm_unit: ViewMut<Unit>,
    mut vm_threat_list: ViewMut<ThreatList>,
    mut vm_melee: ViewMut<Melee>,
    v_guid: View<Guid>,
    v_wpos: View<WorldPosition>,
    v_creature: View<Creature>,
    (mut vm_player, mut vm_health, v_spell): (ViewMut<Player>, ViewMut<Health>, View<SpellCast>),
) {
    (&mut vm_behavior)
        .iter()
        .with_id()
        .for_each(|(entity_id, mut behavior)| {
            let mut context = BTContext {
                entity_id,
                neighbors: behavior.neighbors(),
                dt: &dt,
                map: &map,
                world_context: &world_context,
                vm_movement: &mut vm_movement,
                vm_unit: &mut vm_unit,
                vm_threat_list: &mut vm_threat_list,
                vm_health: &mut vm_health,
                vm_melee: &mut vm_melee,
                vm_player: &mut vm_player,
                v_guid: &v_guid,
                v_wpos: &v_wpos,
                v_creature: &v_creature,
                v_spell: &v_spell,
            };

            behavior.tree().tick(dt.0, &mut context, execute_action);
            behavior.reset_neighbors();
        });
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
    let my_position = &ctx.v_wpos[ctx.entity_id];
    let unit_me = &ctx.vm_unit[ctx.entity_id];

    let mut relevant_neighbors: Vec<(EntityId, f32)> = ctx
        .neighbors
        .iter()
        .filter_map(|&neighbor_entity_id| {
            let neighbor_health = &ctx.vm_health[neighbor_entity_id];
            let neighbor_position = &ctx.v_wpos[neighbor_entity_id];
            let neighbor_unit = &ctx.vm_unit[neighbor_entity_id];
            let neighbor_level = if let Ok(player) = ctx.vm_player.get(neighbor_entity_id) {
                player.level()
            } else if let Ok(other_creature) = ctx.v_creature.get(neighbor_entity_id) {
                other_creature.level_against(creature.real_level())
            } else {
                0
            };

            let neighbor_distance = my_position.distance_to(&neighbor_position, true);

            if neighbor_health.is_alive()
                && unit_me.is_hostile_to(&neighbor_unit)
                && my_position.distance_to(&neighbor_position, true)
                    <= creature.aggro_distance(neighbor_level)
            {
                Some((neighbor_entity_id, neighbor_distance))
            } else {
                None
            }
        })
        .collect();

    relevant_neighbors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    relevant_neighbors.first().map(|&(neighbor, _)| {
        ctx.vm_threat_list[ctx.entity_id].modify_threat(neighbor, 0.);

        if let Ok(player) = ctx.vm_player.get(neighbor) {
            player.set_in_combat_with(my_guid);
        } else if let Ok(mut other_threat_list) = ctx.vm_threat_list.get(neighbor) {
            other_threat_list.modify_threat(ctx.entity_id, 0.);
        }

        return NodeStatus::Success;
    });

    NodeStatus::Failure
}

fn action_attack_in_melee(ctx: &mut BTContext) -> NodeStatus {
    let my_id = ctx.entity_id;

    match Melee::execute_attack(
        my_id,
        ctx.map.0.clone(),
        ctx.world_context.0.data_store.clone(),
        ctx.v_guid,
        ctx.v_wpos,
        ctx.v_spell,
        ctx.v_creature,
        ctx.vm_unit,
        ctx.vm_health,
        ctx.vm_melee,
        ctx.vm_threat_list,
        None,
    ) {
        Ok(_) => return NodeStatus::Success,
        Err(_) => return NodeStatus::Failure,
    }
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
        // Don't change if evading
        let (should_init_movement, should_clear) =
            match ctx.vm_movement[ctx.entity_id].current_movement_kind() {
                MovementKind::Chase {
                    target_entity_id: chasing_entity_id,
                    ..
                } => (*chasing_entity_id != target_entity_id, true),
                MovementKind::ReturnHome => (false, false), // Ignore everything while evading
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
