use shipyard::{Get, ViewMut};

use crate::ecs::components::applied_auras::AppliedAuras;
use crate::ecs::components::spell_cast::{SpellCast, SpellCastSuccess};
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::{
    CmsgCancelAura, CmsgCancelCast, CmsgCastSpell, SmsgCastFailed, SmsgClearExtraAuraInfo,
    SmsgSpellStart,
};
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};
use crate::session::world_session::WSRunnableArgs;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_cast_spell(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgCastSpell = ClientMessage::read_as(data).unwrap();

        session.run(&|WSRunnableArgs { map, .. }| {
            let mut targets = cmsg.cast_targets.clone();

            match SpellCast::cast_spell(
                map.clone(),
                world_context.clone(),
                &session.player_guid().unwrap(),
                cmsg.spell_id,
                &mut targets,
            ) {
                Ok(SpellCastSuccess {
                    spell_base_cast_time,
                }) => {
                    let packet = ServerMessage::new(SmsgClearExtraAuraInfo {
                        target_guid: session.player_guid().unwrap().as_packed(),
                        spell_id: cmsg.spell_id,
                    });

                    session.send(&packet).unwrap();

                    let packet = ServerMessage::new(SmsgSpellStart {
                        caster_entity_guid: session.player_guid().unwrap().as_packed(),
                        caster_unit_guid: session.player_guid().unwrap().as_packed(),
                        spell_id: cmsg.spell_id,
                        cast_id: cmsg.cast_count,
                        cast_flags: 0,
                        cast_time: spell_base_cast_time,
                        target_flags: 0,
                    });

                    session.send(&packet).unwrap();
                }
                Err(result) => {
                    let packet = ServerMessage::new(SmsgCastFailed {
                        spell_id: cmsg.spell_id,
                        result,
                        cast_count: cmsg.cast_count,
                    });

                    session.send(&packet).unwrap();
                }
            }
        });
    }

    pub(crate) fn handle_cmsg_cancel_cast(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgCancelCast = ClientMessage::read_as(data).unwrap();

        if let Some(map) = session.current_map() {
            if let Some(entity_id) = session.player_entity_id() {
                map.world().run(|mut vm_spell: ViewMut<SpellCast>| {
                    if let Some((curr, _)) = vm_spell[entity_id].current_ranged() {
                        if curr.id() == cmsg.spell_id {
                            vm_spell[entity_id].clean();
                        }
                    }
                })
            };
        }
    }

    pub(crate) fn handle_cmsg_cancel_aura(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgCancelAura = ClientMessage::read_as(data).unwrap();

        session.run(&|WSRunnableArgs {
                          map,
                          player_entity_id,
                          ..
                      }| {
            map.world().run(|mut vm_app_auras: ViewMut<AppliedAuras>| {
                if let Ok(mut applied_auras) = (&mut vm_app_auras).get(player_entity_id) {
                    applied_auras.mark_auras_for_removal_by_spell_id(cmsg.spell_id);
                }
            });
        });
    }
}
