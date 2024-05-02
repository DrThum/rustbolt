use std::{sync::Arc, time::Instant};

use shipyard::{AllStoragesViewMut, Get, IntoIter, UniqueView, View, ViewMut};

use crate::{
    datastore::data_types::SpellRecord,
    ecs::components::{guid::Guid, spell_cast::SpellCast},
    entities::player::Player,
    game::{
        map::{Map, WrappedMap},
        spell::Spell,
        spell_effect_handler::{SpellEffectHandler, WrappedSpellEffectHandler},
        world_context::{WorldContext, WrappedWorldContext},
    },
    protocol::{packets::SmsgSpellGo, server::ServerMessage},
    shared::constants::{SpellEffect, MAX_SPELL_EFFECTS},
};

pub fn update_spell(vm_all_storages: AllStoragesViewMut) {
    vm_all_storages.run(
        |map: UniqueView<WrappedMap>,
         world_context: UniqueView<WrappedWorldContext>,
         spell_effect_handler: UniqueView<WrappedSpellEffectHandler>,
         mut vm_spell: ViewMut<SpellCast>,
         v_guid: View<Guid>| {
            if !map.0.has_players() {
                return;
            }

            for (mut spell, guid) in (&mut vm_spell, &v_guid).iter() {
                if let Some((current_ranged, cast_end)) = spell.current_ranged() {
                    let now = Instant::now();

                    if cast_end <= now {
                        let packet = ServerMessage::new(SmsgSpellGo {
                            caster_entity_guid: guid.0.as_packed(),
                            caster_unit_guid: guid.0.as_packed(),
                            spell_id: current_ranged.id(),
                            cast_flags: 0,
                            timestamp: 0, // TODO
                            target_count: 0,
                        });

                        map.0.broadcast_packet(&guid.0, &packet, None, true);

                        handle_effects(
                            world_context.0.clone(),
                            current_ranged.clone(),
                            spell_effect_handler.0.clone(),
                            map.0.clone(),
                            &vm_all_storages,
                        );

                        spell.clean();
                    }
                }
            }
        },
    );
}

fn handle_effects(
    world_context: Arc<WorldContext>,
    spell: Arc<Spell>,
    spell_effect_handler: Arc<SpellEffectHandler>,
    map: Arc<Map>,
    vm_all_storages: &AllStoragesViewMut,
) {
    let spell_record: Arc<SpellRecord> = Arc::new(
        (*world_context
            .data_store
            .get_spell_record(spell.id())
            .unwrap())
        .clone(),
    );

    for effect_index in 0..MAX_SPELL_EFFECTS {
        if let Some(effect) = SpellEffect::n(spell_record.effect[effect_index]) {
            if effect == SpellEffect::None {
                continue;
            }

            let handler = spell_effect_handler.get_handler(&effect);
            handler(
                world_context.clone(),
                spell.clone(),
                map.clone(),
                spell_record.clone(),
                effect_index,
                vm_all_storages,
            );

            // Set player in combat with target if needed
            if effect.is_negative() {
                vm_all_storages.run(|mut vm_player: ViewMut<Player>, guid: View<Guid>| {
                    if let Ok(player) = (&mut vm_player).get(spell.caster()) {
                        if let Ok(target_guid) = guid.get(spell.target()) {
                            if !player.is_in_combat_with(&target_guid.0) {
                                player.set_in_combat_with(target_guid.0);
                            }
                        }
                    }
                });
            }
        }
    }
}
