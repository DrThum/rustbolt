use shipyard::{Get, ViewMut};
use strum::IntoEnumIterator;

use crate::{
    entities::attributes::Attributes,
    shared::constants::{AttributeModifier, AttributeModifierType, UnitAttribute},
};

use super::{AuraEffectHandler, AuraEffectHandlerArgs};

impl AuraEffectHandler {
    pub(super) fn handle_mod_stat(
        AuraEffectHandlerArgs {
            all_storages,
            world_context,
            aura,
            effect_index,
            is_applying,
            ..
        }: AuraEffectHandlerArgs,
    ) {
        all_storages.run(|mut vm_attributes: ViewMut<Attributes>| {
            let spell_record = world_context
                .data_store
                .get_spell_record(aura.spell_id)
                .unwrap();

            let misc_value = spell_record.effect_misc_value[effect_index] as i32;
            let value = spell_record.calc_simple_value(effect_index) as f32; // FIXME: probably not what we want (calc_spell_damage?), see Unit::CalculateSpellDamage in MaNGOS

            if let Ok(mut attr_mods) = (&mut vm_attributes).get(aura.target_id) {
                for stat in UnitAttribute::iter() {
                    // -1 or -2 = all stats
                    if misc_value < 0 || stat as i32 == misc_value {
                        let attr_mod = AttributeModifier::n(stat as i64)
                            .expect("handle_mod_stat: invalid attribute modifier");

                        let amount = if is_applying { value } else { -value };
                        attr_mods.add_modifier(attr_mod, AttributeModifierType::TotalValue, amount);
                    }
                }
            }
        });
    }
}
