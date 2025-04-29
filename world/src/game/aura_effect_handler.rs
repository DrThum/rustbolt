use std::{collections::HashMap, sync::Arc};

use log::{error, trace};
use shipyard::AllStoragesViewMut;

use crate::{create_wrapped_resource, shared::constants::AuraEffect};

use super::{aura::Aura, world_context::WorldContext};

mod stats;

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
}

create_wrapped_resource!(WrappedAuraEffectHandler, AuraEffectHandler);

pub struct AuraEffectHandlerArgs<'a, 'b> {
    pub world_context: Arc<WorldContext>,
    pub all_storages: &'a AllStoragesViewMut<'b>,
    pub aura: &'a mut Aura,
    pub effect_index: usize,
    pub is_applying: bool, // true when applying the aura, false when removing
}
