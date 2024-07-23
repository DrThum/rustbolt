use shipyard::{EntityId, Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::components::{
        behavior::Behavior, guid::Guid, movement::Movement, nearby_players::NearbyPlayers,
        threat_list::ThreatList, unit::Unit, unwind::Unwind,
    },
    entities::{
        creature::Creature,
        game_object::GameObject,
        object_guid::ObjectGuid,
        player::Player,
        position::{Position, WorldPosition},
    },
    game::map::WrappedMap,
};

pub fn unwind_creatures(
    map: UniqueView<WrappedMap>,
    v_guid: View<Guid>,
    v_creature: View<Creature>,
    v_player: View<Player>,
    v_game_object: View<GameObject>,
    (
        mut vm_wpos,
        mut vm_unit,
        mut vm_unwind,
        mut vm_movement,
        mut vm_threat_list,
        mut vm_behavior,
        mut vm_nearby_players,
    ): (
        ViewMut<WorldPosition>,
        ViewMut<Unit>,
        ViewMut<Unwind>,
        ViewMut<Movement>,
        ViewMut<ThreatList>,
        ViewMut<Behavior>,
        ViewMut<NearbyPlayers>,
    ),
) {
    let mut unwinding_update_data: Vec<(ObjectGuid, EntityId, Position)> = Vec::new();
    for (my_entity_id, (guid, me, movement, _)) in
        (&v_guid, &v_creature, &mut vm_movement, &vm_unwind)
            .iter()
            .with_id()
    {
        movement.reset();
        unwinding_update_data.push((guid.0, my_entity_id, me.spawn_position.as_position()));
    }

    // Teleport back to our spawn
    let movement_as_view = vm_movement.as_view();
    for (my_guid, my_entity_id, my_position) in unwinding_update_data {
        map.0.update_entity_position(
            &my_guid,
            my_entity_id,
            None,
            &my_position,
            &movement_as_view,
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

    // Unset in combat with any enemy
    for (my_entity_id, (unit_me, _)) in (&mut vm_unit, &mut vm_unwind).iter().with_id() {
        for (other_entity_id, _) in vm_threat_list[my_entity_id].threat_list() {
            if let Ok(player) = v_player.get(other_entity_id) {
                player.unset_in_combat_with(v_guid[my_entity_id].0);
            } else if let Ok(mut creature_threat) = (&mut vm_threat_list).get(other_entity_id) {
                creature_threat.remove(&my_entity_id);
            }
        }

        unit_me.set_target(None, 0);
        unit_me.set_combat_state(false);
        vm_threat_list[my_entity_id].reset();
    }

    // Start with a blank sheet on the next tick
    vm_unwind.clear();
}
