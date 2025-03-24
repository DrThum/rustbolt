use shipyard::{IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    ecs::components::{
        guid::Guid, melee::Melee, powers::Powers, spell_cast::SpellCast, threat_list::ThreatList,
        unit::Unit,
    },
    entities::{creature::Creature, player::Player, position::WorldPosition},
    game::{
        map::{HasPlayers, WrappedMap},
        world_context::WrappedWorldContext,
    },
};

// TODO: Move to systems/combat?
pub fn attempt_melee_attack(
    (has_players, map, world_context): (
        UniqueView<HasPlayers>,
        UniqueView<WrappedMap>,
        UniqueView<WrappedWorldContext>,
    ),
    v_guid: View<Guid>,
    mut vm_powers: ViewMut<Powers>,
    mut vm_melee: ViewMut<Melee>,
    mut vm_unit: ViewMut<Unit>,
    mut vm_threat_list: ViewMut<ThreatList>,
    mut vm_player: ViewMut<Player>,
    v_wpos: View<WorldPosition>,
    v_spell: View<SpellCast>,
    v_creature: View<Creature>,
) {
    if !**has_players {
        return;
    }

    for (my_id, mut player) in (&mut vm_player).iter().with_id() {
        match Melee::execute_attack(
            my_id,
            map.0.clone(),
            world_context.data_store.clone(),
            &v_guid,
            &v_wpos,
            &v_spell,
            &v_creature,
            &mut vm_unit,
            &mut vm_powers,
            &mut vm_melee,
            &mut vm_threat_list,
            Some(&mut player),
        ) {
            Ok(_) => (),
            Err(_) => continue,
        }
    }
}
