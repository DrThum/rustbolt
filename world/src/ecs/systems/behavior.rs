use shipyard::{
    AllStoragesViewMut, EntityId, Get, IntoIter, IntoWithId, UniqueView, View, ViewMut,
};

use crate::{
    ecs::{
        components::{
            behavior::{Action, Behavior},
            guid::Guid,
            melee::Melee,
            movement::{Movement, MovementKind},
            powers::Powers,
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

pub fn tick(vm_all_storage: AllStoragesViewMut) {
    let dt = vm_all_storage.borrow::<UniqueView<DeltaTime>>().unwrap();
    let map = vm_all_storage.borrow::<UniqueView<WrappedMap>>().unwrap();
    let world_context = vm_all_storage
        .borrow::<UniqueView<WrappedWorldContext>>()
        .unwrap();

    if !map.0.has_players() {
        return;
    }

    vm_all_storage.run(|mut vm_behavior: ViewMut<Behavior>| {
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
                    all_storages: &vm_all_storage,
                };

                behavior.tree().tick(dt.0, &mut context, execute_action);
                behavior.reset_neighbors();
            });
    });
}

fn execute_action(action: &Action, ctx: &mut BTContext) -> NodeStatus {
    match action {
        Action::Aggro => action_aggro(ctx),
        Action::AttackInMelee => action_attack_in_melee(ctx),
        Action::ChaseTarget => action_chase_target(ctx),
        Action::Respawn => action_respawn(ctx),
    }
}

fn action_aggro(ctx: &mut BTContext) -> NodeStatus {
    let aggro_target: Option<EntityId> = ctx.all_storages.run(
        |(v_creature, v_wpos, v_unit, v_powers, v_player): (
            View<Creature>,
            View<WorldPosition>,
            View<Unit>,
            View<Powers>,
            View<Player>,
        )| {
            let creature = &v_creature[ctx.entity_id];
            let my_position = v_wpos[ctx.entity_id];
            let unit_me = &v_unit[ctx.entity_id];

            let mut aggro_target = None;
            let mut current_closest = f32::MAX;

            ctx.neighbors.iter().for_each(|&neighbor_entity_id| {
                let neighbor_powers = &v_powers[neighbor_entity_id];
                let neighbor_position = &v_wpos[neighbor_entity_id];
                let neighbor_unit = &v_unit[neighbor_entity_id];
                let neighbor_level = if let Ok(player) = v_player.get(neighbor_entity_id) {
                    player.level()
                } else if let Ok(other_creature) = v_creature.get(neighbor_entity_id) {
                    other_creature.level_against(creature.real_level())
                } else {
                    0
                };

                let neighbor_distance = my_position.distance_to(&neighbor_position, true);

                if neighbor_powers.is_alive()
                    && unit_me.is_hostile_to(&neighbor_unit)
                    && my_position.distance_to(&neighbor_position, true)
                        <= creature.aggro_distance(neighbor_level)
                    && neighbor_distance < current_closest
                {
                    current_closest = neighbor_distance;
                    aggro_target = Some(neighbor_entity_id);
                }
            });

            aggro_target
        },
    );

    match aggro_target {
        Some(target_entity_id) => {
            ctx.all_storages.run(
                |(v_guid, mut vm_threat_list, v_player): (
                    View<Guid>,
                    ViewMut<ThreatList>,
                    View<Player>,
                )| {
                    let my_guid = v_guid[ctx.entity_id].0;
                    vm_threat_list[ctx.entity_id].modify_threat(target_entity_id, 0.);

                    if let Ok(player) = v_player.get(target_entity_id) {
                        player.set_in_combat_with(my_guid);
                    } else if let Ok(mut other_threat_list) =
                        (&mut vm_threat_list).get(target_entity_id)
                    {
                        other_threat_list.modify_threat(ctx.entity_id, 0.);
                    }
                },
            );

            NodeStatus::Success
        }
        None => NodeStatus::Failure,
    }
}

fn action_attack_in_melee(ctx: &mut BTContext) -> NodeStatus {
    let my_id = ctx.entity_id;

    ctx.all_storages.run(
        |(
            v_guid,
            mut vm_wpos,
            v_spell,
            v_creature,
            mut vm_unit,
            mut vm_powers,
            mut vm_melee,
            mut vm_threat_list,
        ): (
            View<Guid>,
            ViewMut<WorldPosition>,
            View<SpellCast>,
            View<Creature>,
            ViewMut<Unit>,
            ViewMut<Powers>,
            ViewMut<Melee>,
            ViewMut<ThreatList>,
        )| {
            match Melee::execute_attack(
                my_id,
                ctx.map.0.clone(),
                ctx.world_context.0.data_store.clone(),
                &v_guid,
                &mut vm_wpos,
                &v_spell,
                &v_creature,
                &mut vm_unit,
                &mut vm_powers,
                &mut vm_melee,
                &mut vm_threat_list,
                None,
            ) {
                Ok(_) => NodeStatus::Success,
                Err(_) => NodeStatus::Failure,
            }
        },
    )
}

fn action_chase_target(ctx: &mut BTContext) -> NodeStatus {
    ctx.all_storages.run(
        |(v_unit, v_wpos, mut vm_movement, v_guid): (
            View<Unit>,
            View<WorldPosition>,
            ViewMut<Movement>,
            View<Guid>,
        )| {
            let unit_me = &v_unit[ctx.entity_id];
            if let Some(target_entity_id) = unit_me.target() {
                let my_bounding_radius = unit_me.bounding_radius();
                let target_bounding_radius = v_unit[target_entity_id].bounding_radius();
                let chase_distance = my_bounding_radius + target_bounding_radius;
                let my_current_position = v_wpos[ctx.entity_id];
                let target_position = v_wpos[target_entity_id];

                // Change movement if
                // - already chasing but chasing the wrong target
                // - not chasing and the target is too far away from us
                // Don't change if evading
                let (should_init_movement, should_clear) = match vm_movement[ctx.entity_id]
                    .current_movement_kind()
                {
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
                        vm_movement[ctx.entity_id].clear(true);
                    }

                    let speed = vm_movement[ctx.entity_id].speed_run;
                    vm_movement[ctx.entity_id].start_chasing(
                        &v_guid[ctx.entity_id].0,
                        &v_guid[target_entity_id].0,
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
        },
    )
}

fn action_respawn(ctx: &mut BTContext) -> NodeStatus {
    ctx.all_storages.run(
        |(v_creature, v_wpos, v_powers): (View<Creature>, View<WorldPosition>, View<Powers>)| {
            let Ok(creature) = v_creature.get(ctx.entity_id) else {
                return NodeStatus::Failure;
            };

            let mut position = v_wpos[ctx.entity_id];
            position.update_local(&creature.spawn_position.to_position());

            v_powers[ctx.entity_id].heal_to_max();

            NodeStatus::Success
        },
    )
}
