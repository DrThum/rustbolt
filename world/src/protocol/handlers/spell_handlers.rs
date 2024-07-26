use std::sync::Arc;

use log::{error, warn};
use shipyard::{View, ViewMut};

use crate::ecs::components::powers::Powers;
use crate::ecs::components::spell_cast::SpellCast;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::{
    CmsgCancelCast, CmsgCastSpell, SmsgCastFailed, SmsgClearExtraAuraInfo, SmsgSpellStart,
};
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::{WSRunnableArgs, WorldSession};
use crate::shared::constants::SpellFailReason;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_cast_spell(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgCastSpell = ClientMessage::read_as(data).unwrap();
        let mut targets = cmsg.cast_targets.clone();
        targets.update_internal_refs(
            session
                .current_map()
                .expect("received CMSG_CAST_SPELL but player is not on a map")
                .clone(),
        );

        let spell_record = world_context.data_store.get_spell_record(cmsg.spell_id);
        if spell_record.is_none() {
            error!("unknown spell with id {}", cmsg.spell_id);
            return;
        }
        let spell_record = spell_record.unwrap();

        let spell_base_cast_time = spell_record
            .base_cast_time(world_context.data_store.clone())
            .unwrap();

        session.run(&|WSRunnableArgs {
                          map,
                          player_entity_id,
                          ..
                      }| {
            map.world()
                .run(|mut vm_spell: ViewMut<SpellCast>, v_powers: View<Powers>| {
                    if vm_spell[player_entity_id].current_ranged().is_some() {
                        let packet = ServerMessage::new(SmsgCastFailed {
                            spell_id: cmsg.spell_id,
                            result: SpellFailReason::SpellInProgress,
                            cast_count: cmsg.cast_count,
                        });

                        session.send(&packet).unwrap();

                        return;
                    }

                    let Some(spell_record) =
                        world_context.data_store.get_spell_record(cmsg.spell_id)
                    else {
                        warn!("attempt to cast non-existing spell {}", cmsg.spell_id);
                        return;
                    };

                    let powers = &v_powers[player_entity_id];
                    let power_cost = spell_record.calculate_power_cost(
                        powers.base_health(),
                        powers.base_mana(),
                        powers.snapshot(),
                    );

                    vm_spell[player_entity_id].set_current_ranged(
                        cmsg.spell_id,
                        spell_base_cast_time,
                        player_entity_id,
                        targets.unit_target(),
                        targets.game_object_target(),
                        power_cost,
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
        });
    }

    pub(crate) fn handle_cmsg_cancel_cast(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
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
}
