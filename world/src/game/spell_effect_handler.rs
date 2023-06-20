use std::{collections::HashMap, sync::Arc};

use log::{error, trace};
use shipyard::{AllStoragesViewMut, Unique};

use crate::{datastore::data_types::SpellRecord, shared::constants::SpellEffect};

use super::{map::Map, spell::Spell, world_context::WorldContext};

pub type EffectHandler = Box<
    dyn Send
        + Sync
        + for<'a, 'b> Fn(
            Arc<WorldContext>,
            Arc<Spell>,
            Arc<Map>,
            Arc<SpellRecord>,
            usize,
            &'a AllStoragesViewMut<'b>,
        ) -> (),
>;

macro_rules! define_handler {
    ($effect:expr, $handler:expr) => {
        (
            $effect,
            Box::new(
                |world_context,
                 spell,
                 map,
                 record,
                 eff_index,
                 all_storages: &AllStoragesViewMut| {
                    $handler(world_context, spell, map, record, eff_index, all_storages)
                },
            ) as EffectHandler,
        )
    };
}

pub struct SpellEffectHandler {
    handlers: HashMap<SpellEffect, EffectHandler>,
}

impl SpellEffectHandler {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::from([
                define_handler!(SpellEffect::None, SpellEffectHandler::unhandled),
                define_handler!(
                    SpellEffect::SchoolDamage,
                    SpellEffectHandler::handle_effect_school_damage
                ),
            ]),
        }
    }

    pub fn get_handler(&self, spell_effect: &SpellEffect) -> &EffectHandler {
        self.handlers
            .get(&spell_effect)
            .map(|eff| {
                trace!("Handling spell effect {:?}", spell_effect);
                eff
            })
            .unwrap_or_else(|| {
                error!("Unhandled spell effect {:?}", spell_effect);
                self.handlers.get(&SpellEffect::None).unwrap()
            })
    }
}

#[derive(Unique)]
pub struct WrappedSpellEffectHandler(pub Arc<SpellEffectHandler>);
