use shipyard::{Get, IntoIter, UniqueView, View, ViewMut};

use crate::{
    ecs::components::{guid::Guid, powers::Powers, threat_list::ThreatList, unit::Unit},
    entities::player::Player,
    game::map::WrappedMap,
};

pub fn update_combat_state(
    v_player: View<Player>,
    map: UniqueView<WrappedMap>,
    vm_unit: ViewMut<Unit>,
    v_threat_list: View<ThreatList>,
    v_powers: View<Powers>,
) {
    if !map.0.has_players() {
        return;
    }

    for (player, unit, powers) in (&v_player, &vm_unit, &v_powers).iter() {
        if !powers.is_alive() {
            if unit.combat_state() {
                unit.set_combat_state(false);
            }

            player.reset_in_combat_with();

            continue;
        }

        let in_combat_with = player.in_combat_with();
        // We need to update the combat state if we have opponents and are not in combat, or the
        // opposite
        let should_update_combat_state = in_combat_with.is_empty() == unit.combat_state();

        if should_update_combat_state {
            unit.set_combat_state(!unit.combat_state());
        }
    }

    for (unit, threat_list, powers) in (&vm_unit, &v_threat_list, &v_powers).iter() {
        if !powers.is_alive() {
            if unit.combat_state() {
                unit.set_combat_state(false);
            }

            continue;
        }

        // We need to update the combat state if we have threats and are not in combat, or the
        // opposite
        let should_update_combat_state = threat_list.is_empty() == unit.combat_state();

        if should_update_combat_state {
            unit.set_combat_state(!unit.combat_state());
        }
    }
}

// Select the target with the highest threat level
// TODO: 130%/110% rule for taking aggro if there's already a target
pub fn select_target(
    v_powers: View<Powers>,
    map: UniqueView<WrappedMap>,
    mut vm_unit: ViewMut<Unit>,
    mut vm_threat_list: ViewMut<ThreatList>,
    v_guid: View<Guid>,
) {
    if !map.0.has_players() {
        return;
    }

    for (mut unit, mut threat_list, powers) in (&mut vm_unit, &mut vm_threat_list, &v_powers).iter()
    {
        // Reset our target and threat list if we're dead
        if !powers.is_alive() {
            if unit.target().is_some() {
                unit.set_target(None, 0);
            }

            if !threat_list.is_empty() {
                threat_list.reset();
            }

            continue;
        }

        // Remove dead entities from our threat list
        threat_list.threat_list_mut().retain(|&entity_id, _| {
            v_powers
                .get(entity_id)
                .map(|h| h.is_alive())
                .unwrap_or(false)
        });

        threat_list
            .threat_list()
            .into_iter()
            .max_by(|&a, &b| a.1.total_cmp(&b.1))
            .map(|(entity_id, _threat)| {
                if unit.target() != Some(entity_id) {
                    if let Ok(target_guid) = v_guid.get(entity_id) {
                        unit.set_target(Some(entity_id), target_guid.0.raw());
                    } else {
                        threat_list.remove(&entity_id);
                    }
                }
            });
    }
}
