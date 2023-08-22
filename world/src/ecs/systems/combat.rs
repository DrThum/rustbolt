use shipyard::{IntoIter, View, ViewMut};

use crate::{
    ecs::components::{guid::Guid, threat_list::ThreatList, unit::Unit},
    entities::player::Player,
};

pub fn update_combat_state(
    v_player: View<Player>,
    vm_unit: ViewMut<Unit>,
    v_threat_list: View<ThreatList>,
) {
    for (player, unit) in (&v_player, &vm_unit).iter() {
        let in_combat_with = player.in_combat_with();
        // We need to update the combat state if we have opponents and are not in combat, or the
        // opposite
        let should_update_combat_state = in_combat_with.is_empty() == unit.combat_state();

        if should_update_combat_state {
            unit.set_combat_state(!unit.combat_state());
        }
    }

    for (unit, threat_list) in (&vm_unit, &v_threat_list).iter() {
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
    mut vm_unit: ViewMut<Unit>,
    v_threat_list: View<ThreatList>,
    v_guid: View<Guid>,
) {
    for (mut unit, threat_list) in (&mut vm_unit, &v_threat_list).iter() {
        threat_list
            .threat_list()
            .into_iter()
            .max_by(|&a, &b| a.1.total_cmp(&b.1))
            .map(|(entity_id, _threat)| {
                if unit.target() != Some(entity_id) {
                    unit.set_target(Some(entity_id), v_guid[entity_id].0.raw());
                }
            });
    }
}
