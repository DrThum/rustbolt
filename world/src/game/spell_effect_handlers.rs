use super::spell_effect_handler::{SpellEffectHandler, SpellEffectHandlerArgs};

mod combat;
mod misc;

impl SpellEffectHandler {
    pub(crate) fn unhandled(_args: SpellEffectHandlerArgs) {}
}
