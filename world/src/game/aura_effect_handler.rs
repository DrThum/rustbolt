use std::{collections::HashMap, sync::Arc};

use log::{error, trace};
use shipyard::{AllStoragesViewMut, Get, ViewMut};
use strum::IntoEnumIterator;

use crate::{
    create_wrapped_resource,
    entities::attribute_modifiers::AttributeModifiers,
    shared::constants::{AttributeModifier, AttributeModifierType, AuraEffect, UnitAttribute},
};

use super::{aura::Aura, world_context::WorldContext};

type EffectHandler = Box<dyn Send + Sync + for<'a, 'b> Fn(AuraEffectHandlerArgs)>;

macro_rules! define_handler {
    ($effect:expr, $handler: expr) => {
        (
            $effect,
            Box::new(move |args: AuraEffectHandlerArgs| $handler(args)) as EffectHandler,
        )
    };
}

pub struct AuraEffectHandler {
    handlers: HashMap<AuraEffect, EffectHandler>,
}

impl Default for AuraEffectHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl AuraEffectHandler {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::from([
                define_handler!(AuraEffect::None, AuraEffectHandler::unhandled),
                define_handler!(AuraEffect::ModStat, AuraEffectHandler::handle_mod_stat),
            ]),
        }
    }

    pub fn get_handler(&self, aura_effect: &AuraEffect) -> &EffectHandler {
        self.handlers
            .get(aura_effect)
            .map(|eff| {
                trace!("handling aura effect {:?}", aura_effect);
                eff
            })
            .unwrap_or_else(|| {
                error!("unhandled aura effect {:?}", aura_effect);
                self.handlers.get(&AuraEffect::None).unwrap()
            })
    }

    fn unhandled(_args: AuraEffectHandlerArgs) {}

    fn handle_mod_stat(
        AuraEffectHandlerArgs {
            all_storages,
            world_context,
            aura,
            effect_index,
        }: AuraEffectHandlerArgs,
    ) {
        all_storages.run(|mut vm_attribute_modifiers: ViewMut<AttributeModifiers>| {
            let spell_record = world_context
                .data_store
                .get_spell_record(aura.spell_id)
                .unwrap();

            let misc_value = spell_record.effect_misc_value[effect_index] as i32;
            let value = spell_record.calc_simple_value(effect_index) as f32; // FIXME: probably not what we want (calc_spell_damage?), see Unit::CalculateSpellDamage in MaNGOS

            if let Ok(mut attr_mods) = (&mut vm_attribute_modifiers).get(aura.target_id) {
                for stat in UnitAttribute::iter() {
                    // -1 or -2 = all stats
                    if misc_value < 0 || stat as i32 == misc_value {
                        let attr_mod = AttributeModifier::n(stat as i64)
                            .expect("handle_mod_stat: invalid attribute modifier");
                        attr_mods.add_modifier(attr_mod, AttributeModifierType::TotalValue, value);
                    }
                }
            }
        });
    }
}

create_wrapped_resource!(WrappedAuraEffectHandler, AuraEffectHandler);

pub struct AuraEffectHandlerArgs<'a, 'b> {
    pub world_context: Arc<WorldContext>,
    pub all_storages: &'a AllStoragesViewMut<'b>,
    pub aura: &'a mut Aura,
    pub effect_index: usize,
}
