use shipyard::{Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::components::{
        guid::Guid, health::Health, melee::Melee, spell_cast::SpellCast, threat_list::ThreatList,
        unit::Unit,
    },
    entities::{player::Player, position::WorldPosition},
    game::map::WrappedMap,
};

// TODO: Move to systems/combat?
pub fn attempt_melee_attack(
    map: UniqueView<WrappedMap>,
    v_guid: View<Guid>,
    mut vm_health: ViewMut<Health>,
    mut vm_melee: ViewMut<Melee>,
    mut vm_unit: ViewMut<Unit>,
    mut vm_threat_list: ViewMut<ThreatList>,
    v_wpos: View<WorldPosition>,
    v_spell: View<SpellCast>,
    v_player: View<Player>,
) {
    for (my_id, (_, mut unit, _, _)) in (&v_guid, &mut vm_unit, &v_wpos, &v_player).iter().with_id()
    {
        if let Some(target_id) = unit.target() {
            if let Ok(target_guid) = v_guid.get(target_id).map(|g| g.0) {
                match Melee::execute_attack(
                    my_id,
                    target_id,
                    target_guid,
                    map.0.clone(),
                    &v_guid,
                    &v_wpos,
                    &v_spell,
                    &mut vm_health,
                    &mut vm_melee,
                    &mut vm_threat_list,
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
