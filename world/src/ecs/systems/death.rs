use shipyard::{Get, UniqueView, UniqueViewMut, View, ViewMut};

use crate::{
    datastore::data_types::MapRecord,
    ecs::{
        components::{guid::Guid, unit::Unit},
        resources::{CombatEvents, UnitDied},
    },
    entities::{attributes::Attributes, creature::Creature, player::Player},
    game::experience::Experience,
    shared::constants::UnitDynamicFlag,
};

pub fn resolve_deaths(
    map_record: UniqueView<MapRecord>,
    v_creature: View<Creature>,
    v_unit: View<Unit>,
    v_guid: View<Guid>,
    mut vm_attributes: ViewMut<Attributes>,
    mut combat_events: UniqueViewMut<CombatEvents>,
    mut vm_player: ViewMut<Player>,
) {
    let unit_died_events = combat_events.drain();

    for UnitDied { killer, victim } in unit_died_events {
        let Ok(mut player) = (&mut vm_player).get(killer) else {
            continue;
        };

        let Ok(target_guid) = v_guid.get(victim) else {
            continue;
        };

        let mut has_loot = false; // TODO: Handle player case (Insignia looting in PvP)
        if let Ok(creature) = v_creature.get(victim) {
            let Ok(mut attributes) = (&mut vm_attributes).get(killer) else {
                continue;
            };

            let xp_gain = Experience::xp_gain_against(&player, creature, &map_record);
            player.give_experience(xp_gain, Some(**target_guid), &mut attributes);
            player.notify_killed_creature(creature.guid(), creature.template.entry);

            has_loot = creature.generate_loot();
        }

        if let Ok(target_unit) = v_unit.get(victim) {
            if has_loot {
                target_unit.set_dynamic_flag(UnitDynamicFlag::Lootable);
            }
        }

        player.unset_in_combat_with(**target_guid);
    }
}
