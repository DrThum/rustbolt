use std::time::Instant;

use shipyard::{IntoIter, UniqueView, View, ViewMut};

use crate::{
    ecs::components::{guid::Guid, spell_cast::SpellCast},
    game::map::WrappedMap,
    protocol::{packets::SmsgSpellGo, server::ServerMessage},
};

pub fn update_spell(
    mut vm_spell: ViewMut<SpellCast>,
    v_guid: View<Guid>,
    map: UniqueView<WrappedMap>,
) {
    for (mut spell, guid) in (&mut vm_spell, &v_guid).iter() {
        if let Some((current_ranged, cast_end)) = spell.current_ranged() {
            let now = Instant::now();

            if cast_end <= now {
                let packet = ServerMessage::new(SmsgSpellGo {
                    caster_entity_guid: guid.0.as_packed(),
                    caster_unit_guid: guid.0.as_packed(),
                    spell_id: current_ranged,
                    cast_flags: 0,
                    timestamp: 0, // TODO
                    target_count: 0,
                });

                map.0.broadcast_packet(&guid.0, &packet, None, true);

                spell.clean();
            }
        }
    }
}
