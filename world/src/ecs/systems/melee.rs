use shipyard::{Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::components::{
        guid::Guid, health::Health, melee::Melee, spell_cast::SpellCast, threat_list::ThreatList,
        unit::Unit,
    },
    entities::{creature::Creature, player::Player, position::WorldPosition},
    game::{map::WrappedMap, world_context::WrappedWorldContext},
};

// TODO: Move to systems/combat?
pub fn attempt_melee_attack(
    (map, world_context): (UniqueView<WrappedMap>, UniqueView<WrappedWorldContext>),
    v_guid: View<Guid>,
    mut vm_health: ViewMut<Health>,
    mut vm_melee: ViewMut<Melee>,
    mut vm_unit: ViewMut<Unit>,
    mut vm_threat_list: ViewMut<ThreatList>,
    mut vm_player: ViewMut<Player>,
    v_wpos: View<WorldPosition>,
    v_spell: View<SpellCast>,
    v_creature: View<Creature>,
) {
    for (my_id, (_, mut unit, _)) in (&v_guid, &mut vm_unit, &v_wpos).iter().with_id() {
        if vm_player.get(my_id).is_err() {
            continue;
        }

        if let Some(target_id) = unit.target() {
            if let Ok(target_guid) = v_guid.get(target_id).map(|g| g.0) {
                match Melee::execute_attack(
                    my_id,
                    target_id,
                    target_guid,
                    map.0.clone(),
                    world_context.0.data_store.clone(),
                    &v_guid,
                    &v_wpos,
                    &v_spell,
                    &v_creature,
                    &mut vm_health,
                    &mut vm_melee,
                    &mut vm_threat_list,
                    &mut vm_player,
                ) {
                    Ok(_) => (),
                    Err(_) => continue,
                }
            } else {
                unit.set_target(None, 0);
            }
        }
    }
}
