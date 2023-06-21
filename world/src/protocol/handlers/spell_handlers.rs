use std::sync::Arc;

use log::error;
use shipyard::{View, ViewMut};

use crate::ecs::components::spell_cast::SpellCast;
use crate::ecs::components::unit::Unit;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::{
    CmsgCastSpell, SmsgCastFailed, SmsgClearExtraAuraInfo, SmsgSpellStart,
};
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;
use crate::shared::constants::SpellFailReason;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_cast_spell(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgCastSpell = ClientMessage::read_as(data).unwrap();

        let spell_record = world_context.data_store.get_spell_record(cmsg.spell_id);
        if spell_record.is_none() {
            error!("unknown spell with id {}", cmsg.spell_id);
            return;
        }
        let spell_record = spell_record.unwrap();

        let spell_base_cast_time = spell_record
            .base_cast_time(world_context.data_store.clone())
            .unwrap();

        if let Some(map) = session.current_map() {
            if let Some(entity_id) = session.player_entity_id() {
                map.world()
                    .run(|mut vm_spell: ViewMut<SpellCast>, v_unit: View<Unit>| {
                        if vm_spell[entity_id].current_ranged().is_some() {
                            let packet = ServerMessage::new(SmsgCastFailed {
                                spell_id: cmsg.spell_id,
                                result: SpellFailReason::SpellInProgress,
                                cast_count: cmsg.cast_count,
                            });

                            session.send(&packet).unwrap();

                            return;
                        }

                        let target = v_unit[entity_id].target().unwrap_or(entity_id); // Target self by default
                        vm_spell[entity_id].set_current_ranged(
                            cmsg.spell_id,
                            spell_base_cast_time,
                            target,
                        );

                        let packet = ServerMessage::new(SmsgClearExtraAuraInfo {
                            caster_guid: session.player_guid().unwrap().as_packed(),
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
                    });
            }
        }
    }
}
