use shipyard::{IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    datastore::data_types::MapRecord,
    ecs::components::{
        guid::Guid, melee::Melee, powers::Powers, spell_cast::SpellCast, threat_list::ThreatList,
        unit::Unit,
    },
    entities::{
        attributes::Attributes, creature::Creature, player::Player,
        position::WorldPosition,
    },
    game::{map::HasPlayers, packet_broadcaster::WrappedPacketBroadcaster},
    session::session_holder::WrappedSessionHolder,
};

// TODO: Move to systems/combat?
pub fn attempt_melee_attack(
    (has_players, map_record, packet_broadcaster, session_holder): (
        UniqueView<HasPlayers>,
        UniqueView<MapRecord>,
        UniqueView<WrappedPacketBroadcaster>,
        UniqueView<WrappedSessionHolder>,
    ),
    v_guid: View<Guid>,
    mut vm_powers: ViewMut<Powers>,
    mut vm_melee: ViewMut<Melee>,
    mut vm_unit: ViewMut<Unit>,
    mut vm_threat_list: ViewMut<ThreatList>,
    (mut vm_player, mut vm_attributes): (ViewMut<Player>, ViewMut<Attributes>),
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
            (**packet_broadcaster).clone(),
            (**session_holder).clone(),
            &map_record,
            &v_guid,
            &v_wpos,
            &v_spell,
            &v_creature,
            &mut vm_unit,
            &mut vm_powers,
            &mut vm_melee,
            &mut vm_threat_list,
            &mut vm_attributes,
            Some(&mut player),
        ) {
            Ok(_) => (),
            Err(_) => continue,
        }
    }
}
