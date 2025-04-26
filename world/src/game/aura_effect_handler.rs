use std::{collections::HashMap, sync::Arc};

use log::{error, trace};
use shipyard::AllStoragesViewMut;

use crate::{create_wrapped_resource, shared::constants::AuraEffect};

use super::world_context::WorldContext;

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
            handlers: HashMap::from([define_handler!(
                AuraEffect::None,
                AuraEffectHandler::unhandled
            )]),
        }
    }

    pub fn get_handler(&self, aura_effect: &AuraEffect) -> &EffectHandler {
        self.handlers
            .get(aura_effect)
            .map(|eff| {
                trace!("Handling aura effect {:?}", aura_effect);
                eff
            })
            .unwrap_or_else(|| {
                error!("Unhandled aura effect {:?}", aura_effect);
                self.handlers.get(&AuraEffect::None).unwrap()
            })
    }

    fn unhandled(_args: AuraEffectHandlerArgs) {}
}

create_wrapped_resource!(WrappedAuraEffectHandler, AuraEffectHandler);

pub struct AuraEffectHandlerArgs<'a, 'b> {
    pub world_context: Arc<WorldContext>,
    pub all_storages: &'a AllStoragesViewMut<'b>,
}
