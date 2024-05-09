use shipyard::{IntoIter, UniqueView, View, ViewMut};

use crate::{
    ecs::components::guid::Guid,
    entities::{
        internal_values::WrappedInternalValues,
        player::Player,
        position::WorldPosition,
        update::{UpdateBlockBuilder, UpdateData, UpdateType},
    },
    game::map::WrappedMap,
    protocol::packets::SmsgUpdateObject,
    shared::constants::{AttributeModifier, SpellSchool, UnitAttribute},
};

pub fn send_entity_update(
    map: UniqueView<WrappedMap>,
    v_guid: View<Guid>,
    v_int_vals: View<WrappedInternalValues>,
    v_wpos: View<WorldPosition>,
) {
    if !map.0.has_players() {
        return;
    }

    for (guid, wrapped_int_vals, wpos) in (&v_guid, &v_int_vals, &v_wpos).iter() {
        let mut internal_values = wrapped_int_vals.0.write();
        if internal_values.has_dirty() {
            for session in map.0.sessions_nearby_position(
                &wpos.to_position(),
                map.0.visibility_distance(),
                true,
                None,
            ) {
                let mut update_builder = UpdateBlockBuilder::new();

                for index in internal_values.get_dirty_indexes() {
                    let value = internal_values.get_u32(index as usize);
                    update_builder.add(index as usize, value);
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

pub fn update_attributes_from_modifiers(
    map: UniqueView<WrappedMap>,
    mut vm_player: ViewMut<Player>,
) {
    if !map.0.has_players() {
        return;
    }

    for mut player in (&mut vm_player).iter() {
        let mut attributes_to_update: Vec<(UnitAttribute, u32)> = Vec::new();
        let mut resistances_to_update: Vec<(SpellSchool, u32)> = Vec::new();

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
                    AttributeModifier::Health => todo!(),
                    AttributeModifier::Mana => todo!(),
                    AttributeModifier::Rage => todo!(),
                    AttributeModifier::Focus => todo!(),
                    AttributeModifier::Energy => todo!(),
                    AttributeModifier::Happiness => todo!(),
                    AttributeModifier::Armor => resistances_to_update.push((
                        SpellSchool::Normal,
                        attr_mods.total_modifier_value(AttributeModifier::Armor) as u32,
                    )),
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
                    AttributeModifier::Max => todo!(),
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
    }
}
