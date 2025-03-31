use std::{sync::Arc, time::Instant};

use shipyard::{AllStoragesViewMut, Get, IntoIter, IntoWithId, UniqueView, View, ViewMut};

use crate::{
    datastore::data_types::{MapRecord, SpellRecord},
    ecs::components::{
        guid::Guid, nearby_players::NearbyPlayers, powers::Powers, spell_cast::SpellCast,
    },
    entities::player::Player,
    game::{
        map::HasPlayers,
        packet_broadcaster::WrappedPacketBroadcaster,
        spell::Spell,
        spell_effect_handler::{SpellEffectHandler, WrappedSpellEffectHandler},
        world_context::{WorldContext, WrappedWorldContext},
    },
    protocol::{packets::SmsgSpellGo, server::ServerMessage},
    shared::constants::{PowerType, SpellEffect, MAX_SPELL_EFFECTS},
};

pub fn update_spell(vm_all_storages: AllStoragesViewMut) {
    vm_all_storages.run(
        |map_record: UniqueView<MapRecord>,
         packet_broadcaster: UniqueView<WrappedPacketBroadcaster>,
         has_players: UniqueView<HasPlayers>,
         world_context: UniqueView<WrappedWorldContext>,
         spell_effect_handler: UniqueView<WrappedSpellEffectHandler>,
         mut vm_spell: ViewMut<SpellCast>,
         v_guid: View<Guid>,
         v_nearby_players: View<NearbyPlayers>| {
            if !**has_players {
                return;
            }

            for (caster_entity_id, (spell, guid, _)) in
                (&mut vm_spell, &v_guid, &v_nearby_players).iter().with_id()
            {
                if let Some((current_ranged, cast_end)) = spell.current_ranged() {
                    let now = Instant::now();

                    if cast_end <= now {
                        let spell_record = world_context
                            .data_store
                            .get_spell_record(current_ranged.id())
                            .expect("unknown spell at end of cast?!");

                        // Take power
                        if current_ranged.power_cost() > 0 {
                            if let Ok((v_powers, mut vm_player)) =
                                vm_all_storages.borrow::<(View<Powers>, ViewMut<Player>)>()
                            {
                                if let Ok(powers) = v_powers.get(caster_entity_id) {
                                    match spell_record.power_type {
                                        PowerType::Health => todo!(),
                                        power_type => {
                                            powers.modify_power(
                                                &power_type,
                                                -(current_ranged.power_cost() as i32),
                                            );
                                        }
                                    }

                                    // Five Seconds Rule (FSR)
                                    if spell_record.power_type == PowerType::Mana {
                                        let _ = (&mut vm_player).get(caster_entity_id).map(
                                            |mut player| {
                                                player.set_has_cast_recently();
                                            },
                                        );
                                    }
                                }
                            }
                        }

                        let packet = ServerMessage::new(SmsgSpellGo {
                            caster_entity_guid: guid.0.as_packed(),
                            caster_unit_guid: guid.0.as_packed(),
                            spell_id: current_ranged.id(),
                            cast_flags: 0,
                            timestamp: 0, // TODO
                            target_count: 0,
                        });

                        packet_broadcaster.broadcast_packet(&guid.0, &packet, None, true);

                        handle_effects(
                            world_context.clone(),
                            current_ranged.clone(),
                            spell_effect_handler.clone(),
                            &map_record,
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
    map_record: &MapRecord,
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
                map_record,
                spell_record.clone(),
                effect_index,
                vm_all_storages,
            );

            // Set player in combat with target if needed
            if effect.is_negative() {
                vm_all_storages.run(
                    |mut vm_player: ViewMut<Player>, v_guid: View<Guid>, v_powers: View<Powers>| {
                        let Some(unit_target) = spell.unit_target() else {
                            return;
                        };

                        let Ok(target_powers) = v_powers.get(unit_target) else {
                            return;
                        };

                        if !target_powers.is_alive() {
                            return;
                        }

                        let Ok(player) = (&mut vm_player).get(spell.caster()) else {
                            return;
                        };
                        let Ok(target_guid) = v_guid.get(unit_target) else {
                            return;
                        };

                        if !player.is_in_combat_with(&target_guid.0) {
                            player.set_in_combat_with(target_guid.0);
                        }
                    },
                );
            }
        }
    }
}
