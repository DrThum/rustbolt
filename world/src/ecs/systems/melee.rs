use shipyard::{IntoIter, IntoWithId, UniqueView, View, ViewMut};

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
    for (my_id, mut player) in (&mut vm_player).iter().with_id() {
        match Melee::execute_attack(
            my_id,
            map.0.clone(),
            world_context.0.data_store.clone(),
            &v_guid,
            &v_wpos,
            &v_spell,
            &v_creature,
            &mut vm_unit,
            &mut vm_health,
            &mut vm_melee,
            &mut vm_threat_list,
            Some(&mut player),
        ) {
            Ok(_) => (),
            Err(_) => continue,
        }
    }
}
