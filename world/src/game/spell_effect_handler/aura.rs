use log::error;
use shipyard::{Get, UniqueView, ViewMut};

use crate::{
    ecs::components::applied_auras::AppliedAuras, session::session_holder::WrappedSessionHolder,
};

use super::{SpellEffectHandler, SpellEffectHandlerArgs};

impl SpellEffectHandler {
    pub fn handle_effect_apply_aura(
        SpellEffectHandlerArgs {
            spell,
            spell_record,
            all_storages,
            world_context,
            ..
        }: SpellEffectHandlerArgs,
    ) {
        all_storages.run(
            |mut vm_app_auras: ViewMut<AppliedAuras>,
             session_holder: UniqueView<WrappedSessionHolder>| {
                let Some(target_entity_id) = spell.unit_target() else {
                    error!("handle_effect_apply_aura: spell has no unit target");
                    return;
                };

                if let Ok(mut applied_auras) = (&mut vm_app_auras).get(target_entity_id) {
                    applied_auras.add_aura(
                        spell.clone(),
                        spell_record.clone(),
                        session_holder.get_session(&spell.caster_guid()),
                        world_context.data_store.clone(),
                    );
                }
            },
        )
    }
}
