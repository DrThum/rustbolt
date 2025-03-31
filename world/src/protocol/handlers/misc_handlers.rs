use crate::ecs::components::spell_cast::SpellCast;
use crate::ecs::components::unit::Unit;
use crate::entities::object_guid::ObjectGuid;
use crate::game::spell_cast_target::SpellCastTargets;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};
use crate::session::world_session::WSRunnableArgs;
use crate::shared::constants::RemarkableSpells;
use log::error;
use shipyard::ViewMut;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_realm_split(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg_realm_split: CmsgRealmSplit = ClientMessage::read_as(data).unwrap();

        let packet = ServerMessage::new(SmsgRealmSplit {
            client_state: cmsg_realm_split.client_state,
            realm_state: 0x00,
            split_date: binrw::NullString::from("01/01/01"),
        });

        session.send(&packet).unwrap();
    }

    pub(crate) fn handle_cmsg_set_selection(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgSetSelection = ClientMessage::read_as(data).unwrap();

        if let Some(map) = session.current_map() {
            if let Some(player_ecs_entity) = map.lookup_entity_ecs(&session.player_guid().unwrap())
            {
                let target_ecs_entity =
                    map.lookup_entity_ecs(&ObjectGuid::from_raw(cmsg.guid).unwrap());

                map.world().run(|mut vm_unit: ViewMut<Unit>| {
                    vm_unit[player_ecs_entity].set_target(target_ecs_entity, cmsg.guid);
                });
            }
        }
    }

    pub fn handle_cmsg_binder_activate(
        PacketHandlerArgs {
            data,
            session,
            world_context,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgBinderActivate = ClientMessage::read_as(data).unwrap();

        session.run(&|WSRunnableArgs { map, .. }| {
            match world_context.data_store.get_map_record(map.id()) {
                None => {
                    error!("handle_cmsg_binder_activate: unknown map in DBC");
                    return;
                }
                Some(map_record) => {
                    if map_record.is_instanceable() {
                        error!("handle_cmsg_binder_activate: player can only bind themselves on a non-instanceable map");
                        return;
                    }
                }
            };

            let mut targets = &mut SpellCastTargets::new_unit(session.player_guid().unwrap());
            match SpellCast::cast_spell(map.clone(), world_context.clone(), &cmsg.guid, RemarkableSpells::Bind as u32, &mut targets) {
                Ok(_) => {
                    let packet = ServerMessage::new(SmsgTrainerBuySucceeded {
                        trainer_guid: cmsg.guid,
                        spell_id: RemarkableSpells::Bind as u32,
                    });

                    session.send(&packet).unwrap();
                },
                Err(fail_reason) => {
                    error!("handle_cmsg_binder_active: innkeeper failed to cast Bind ({fail_reason:?})");
                },
            }
        });
    }
}
