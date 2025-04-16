use std::{collections::HashMap, sync::Arc};

use log::{error, trace};
use shipyard::AllStoragesViewMut;

use crate::{
    create_wrapped_resource,
    datastore::data_types::{MapRecord, SpellRecord},
    shared::constants::SpellEffect,
};

use super::{spell::Spell, world_context::WorldContext};

mod aura;
mod combat;
mod misc;

pub type EffectHandler = Box<dyn Send + Sync + for<'a, 'b> Fn(SpellEffectHandlerArgs)>;

macro_rules! define_handler {
    ($effect:expr, $handler:expr) => {
        (
            $effect,
            Box::new(move |args: SpellEffectHandlerArgs| $handler(args)) as EffectHandler,
        )
    };
}

pub struct SpellEffectHandler {
    handlers: HashMap<SpellEffect, EffectHandler>,
}

impl Default for SpellEffectHandler {
    fn default() -> Self {
        Self::new()
    }
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
                define_handler!(
                    SpellEffect::TeleportUnits,
                    SpellEffectHandler::handle_effect_teleport_units
                ),
                define_handler!(
                    SpellEffect::ApplyAura,
                    SpellEffectHandler::handle_effect_apply_aura
                ),
                define_handler!(SpellEffect::Heal, SpellEffectHandler::handle_effect_heal),
                define_handler!(
                    SpellEffect::OpenLock,
                    SpellEffectHandler::handle_effect_open_lock
                ),
                define_handler!(
                    SpellEffect::OpenLockItem,
                    SpellEffectHandler::handle_effect_open_lock
                ),
                define_handler!(SpellEffect::Bind, SpellEffectHandler::handle_effect_bind),
            ]),
        }
    }

    pub fn get_handler(&self, spell_effect: &SpellEffect) -> &EffectHandler {
        self.handlers
            .get(spell_effect)
            .map(|eff| {
                trace!("Handling spell effect {:?}", spell_effect);
                eff
            })
            .unwrap_or_else(|| {
                error!("Unhandled spell effect {:?}", spell_effect);
                self.handlers.get(&SpellEffect::None).unwrap()
            })
    }

    pub(crate) fn unhandled(_args: SpellEffectHandlerArgs) {}
}

create_wrapped_resource!(WrappedSpellEffectHandler, SpellEffectHandler);

pub struct SpellEffectHandlerArgs<'a, 'b> {
    pub world_context: Arc<WorldContext>,
    pub spell: Arc<Spell>,
    pub map_record: &'a MapRecord,
    pub spell_record: Arc<SpellRecord>,
    pub effect_index: usize,
    pub all_storages: &'a AllStoragesViewMut<'b>,
}
