use std::sync::atomic::Ordering;

use shipyard::{Get, IntoIter, UniqueView, View};

use crate::{
    ecs::components::{guid::Guid, nearby_players::NearbyPlayers},
    entities::{
        game_object::GameObject,
        internal_values::WrappedInternalValues,
        player::Player,
        position::WorldPosition,
        update::{UpdateBlockBuilder, UpdateData, UpdateType},
    },
    game::map::{HasPlayers, WrappedMap},
    protocol::packets::SmsgUpdateObject,
    shared::constants::{AttributeModifier, PowerType, SpellSchool, UnitAttribute},
};

pub fn send_entity_update(
    map: UniqueView<WrappedMap>,
    has_players: UniqueView<HasPlayers>,
    v_guid: View<Guid>,
    v_int_vals: View<WrappedInternalValues>,
    v_wpos: View<WorldPosition>,
    v_nearby_players: View<NearbyPlayers>,
) {
    if !has_players.0 {
        return;
    }

    for (guid, wrapped_int_vals, wpos, _) in
        (&v_guid, &v_int_vals, &v_wpos, &v_nearby_players).iter()
    {
        let mut internal_values = wrapped_int_vals.0.write();
        if internal_values.has_dirty() {
            for session in map.0.sessions_nearby_position(
                &wpos.as_position(),
                map.0.visibility_distance(),
                true,
                None,
            ) {
                let mut update_builder = UpdateBlockBuilder::new();

                for index in internal_values.get_dirty_indexes() {
                    let value = internal_values.get_u32(index);
                    update_builder.add(index, value);
                }

                let blocks = update_builder.build();

                let update_data = vec![UpdateData {
                    update_type: UpdateType::Values,
                    packed_guid: guid.0.as_packed(),
                    blocks,
                }];

                let smsg_update_object = SmsgUpdateObject {
                    updates_count: update_data.len() as u32,
                    has_transport: false,
                    updates: update_data,
                };

                session.update_entity(smsg_update_object);
            }

            internal_values.reset_dirty();
        }
    }
}

pub fn update_player_environment(
    map: UniqueView<WrappedMap>,
    has_players: UniqueView<HasPlayers>,
    v_player: View<Player>,
    v_game_object: View<GameObject>,
) {
    if !has_players.0 {
        return;
    }

    for player in v_player.iter() {
        update_player_attribute_modifiers(player);

        if player
            .needs_nearby_game_objects_refresh
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            for guid in &*player.session.known_guids() {
                let Some(entity_id) = map.0.lookup_entity_ecs(guid) else {
                    continue;
                };

                let Ok(game_object) = v_game_object.get(entity_id) else {
                    continue;
                };

                let packet = game_object.build_update_for_player(player);

                player.session.update_entity(packet);
            }
        }
    }
}

fn update_player_attribute_modifiers(player: &Player) {
    let mut attributes_to_update: Vec<(UnitAttribute, u32)> = Vec::new();
    let mut resistances_to_update: Vec<(SpellSchool, u32)> = Vec::new();
    let mut updated_max_health: Option<u32> = None;
    let mut updated_max_mana: Option<u32> = None;

    {
        let mut attr_mods = player.attribute_modifiers.write();
        for dirty_attr_mod in attr_mods.dirty_modifiers() {
            match dirty_attr_mod {
                AttributeModifier::StatStrength => {
                    attributes_to_update.push((
                        UnitAttribute::Strength,
                        attr_mods.total_modifier_value(AttributeModifier::StatStrength) as u32,
                    ));
                }
                AttributeModifier::StatAgility => {
                    attributes_to_update.push((
                        UnitAttribute::Agility,
                        attr_mods.total_modifier_value(AttributeModifier::StatAgility) as u32,
                    ));
                }
                AttributeModifier::StatStamina => {
                    attributes_to_update.push((
                        UnitAttribute::Stamina,
                        attr_mods.total_modifier_value(AttributeModifier::StatStamina) as u32,
                    ));
                }
                AttributeModifier::StatIntellect => {
                    attributes_to_update.push((
                        UnitAttribute::Intellect,
                        attr_mods.total_modifier_value(AttributeModifier::StatIntellect) as u32,
                    ));
                }
                AttributeModifier::StatSpirit => {
                    attributes_to_update.push((
                        UnitAttribute::Spirit,
                        attr_mods.total_modifier_value(AttributeModifier::StatSpirit) as u32,
                    ));
                }
                AttributeModifier::Health => {
                    let [base, base_percent, total, total_percent] =
                        attr_mods.modifier_values(AttributeModifier::Health);
                    let stamina = attr_mods.total_modifier_value(AttributeModifier::StatStamina);

                    // Add Stamina bonus to Health
                    let bonus_from_stamina = {
                        let base_stamina = stamina.min(20.);
                        let extra_stamina = stamina - base_stamina;
                        base_stamina + (extra_stamina * 10.)
                    };

                    let max_health =
                        ((base * base_percent) + bonus_from_stamina + total) * total_percent;

                    updated_max_health = Some(max_health as u32);
                }
                AttributeModifier::Mana => {
                    let [base, base_percent, total, total_percent] =
                        attr_mods.modifier_values(AttributeModifier::Mana);
                    let intellect =
                        attr_mods.total_modifier_value(AttributeModifier::StatIntellect);

                    // Add Intellect bonus to Mana
                    let bonus_from_intel = {
                        let base_intel = intellect.min(20.);
                        let extra_intel = intellect - base_intel;
                        base_intel + (extra_intel * 15.)
                    };

                    let max_mana =
                        ((base * base_percent) + bonus_from_intel + total) * total_percent;

                    updated_max_mana = Some(max_mana as u32);
                }
                AttributeModifier::Rage => todo!(),
                AttributeModifier::Focus => todo!(),
                AttributeModifier::Energy => todo!(),
                AttributeModifier::Happiness => todo!(),
                AttributeModifier::Armor => {
                    let [base, base_percent, total, total_percent] =
                        attr_mods.modifier_values(AttributeModifier::Armor);
                    let agility = attr_mods.total_modifier_value(AttributeModifier::StatAgility);

                    // Add 2x Agility to the total armor
                    let total_armor =
                        ((base * base_percent) + (agility * 2.) + total) * total_percent;

                    resistances_to_update.push((SpellSchool::Normal, total_armor as u32));
                }
                AttributeModifier::ResistanceHoly => resistances_to_update.push((
                    SpellSchool::Holy,
                    attr_mods.total_modifier_value(AttributeModifier::ResistanceHoly) as u32,
                )),
                AttributeModifier::ResistanceFire => resistances_to_update.push((
                    SpellSchool::Fire,
                    attr_mods.total_modifier_value(AttributeModifier::ResistanceFire) as u32,
                )),
                AttributeModifier::ResistanceNature => resistances_to_update.push((
                    SpellSchool::Nature,
                    attr_mods.total_modifier_value(AttributeModifier::ResistanceNature) as u32,
                )),
                AttributeModifier::ResistanceFrost => resistances_to_update.push((
                    SpellSchool::Frost,
                    attr_mods.total_modifier_value(AttributeModifier::ResistanceFrost) as u32,
                )),
                AttributeModifier::ResistanceShadow => resistances_to_update.push((
                    SpellSchool::Shadow,
                    attr_mods.total_modifier_value(AttributeModifier::ResistanceNature) as u32,
                )),
                AttributeModifier::ResistanceArcane => resistances_to_update.push((
                    SpellSchool::Arcane,
                    attr_mods.total_modifier_value(AttributeModifier::ResistanceArcane) as u32,
                )),
                AttributeModifier::AttackPower => todo!(),
                AttributeModifier::AttackPowerRanged => todo!(),
                AttributeModifier::DamageMainHand => todo!(),
                AttributeModifier::DamageOffHand => todo!(),
                AttributeModifier::DamageRanged => todo!(),
                AttributeModifier::Max => (),
            }
        }

        attr_mods.reset_dirty();
    }

    for (unit_attr, value) in attributes_to_update {
        player.set_attribute(unit_attr, value);
    }

    for (spell_school, value) in resistances_to_update {
        player.set_resistance(spell_school, value);
    }

    if let Some(health) = updated_max_health {
        player.set_max_health(health)
    }
    if let Some(mana) = updated_max_mana {
        player.set_max_power(PowerType::Mana, mana)
    }

    {
        let mut has_just_leveled_up = player.has_just_leveled_up.lock();
        if *has_just_leveled_up {
            player.set_health_to_max();
            player.set_mana_to_max();
            *has_just_leveled_up = false;
        }
    }

    player.calculate_mana_regen();
}
