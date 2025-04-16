use log::error;
use shipyard::{Get, UniqueView, View, ViewMut};

use crate::{
    ecs::components::applied_auras::AppliedAuras, entities::player::Player,
    session::session_holder::WrappedSessionHolder,
};

use super::{SpellEffectHandler, SpellEffectHandlerArgs};

impl SpellEffectHandler {
    pub fn handle_effect_apply_aura(
        SpellEffectHandlerArgs {
            spell,
            spell_record,
            all_storages,
            world_context,
            effect_index,
            ..
        }: SpellEffectHandlerArgs,
    ) {
        all_storages.run(
            |mut vm_app_auras: ViewMut<AppliedAuras>,
             session_holder: UniqueView<WrappedSessionHolder>,
             v_player: View<Player>| {
                let Some(target_entity_id) = spell.unit_target() else {
                    error!("handle_effect_apply_aura: spell has no unit target");
                    return;
                };

                let Ok(mut applied_auras) = (&mut vm_app_auras).get(target_entity_id) else {
                    error!("handle_effect_apply_aura: no AppliedAuras component found on target");
                    return;
                };

                let player = v_player.get(spell.caster()).ok();

                applied_auras.add_aura(
                    spell.clone(),
                    effect_index,
                    spell_record.clone(),
                    player.map(|p| p.session.clone()),
                    spell
                        .unit_target_guid()
                        .and_then(|target_guid| session_holder.get_session(&target_guid)),
                    world_context.data_store.clone(),
                );
            },
        )
    }
}
