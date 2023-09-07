use std::sync::Arc;

use shipyard::{Get, View, ViewMut};

use crate::ecs::components::unit::Unit;
use crate::entities::creature::Creature;
use crate::entities::object_guid::ObjectGuid;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;
use crate::shared::constants::{LootType, UnitFlags};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_loot(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgLoot = ClientMessage::read_as(data).unwrap();

        if let Some(target_guid) = ObjectGuid::from_raw(cmsg.target_guid) {
            if let Some(map) = session.current_map() {
                let maybe_loot = map.world().run(|v_creature: View<Creature>| {
                    map.lookup_entity_ecs(&target_guid)
                        .and_then(|looted_entity_id| {
                            v_creature
                                .get(looted_entity_id)
                                .ok()
                                .and_then(|creature| creature.current_loot())
                        })
                });

                if let Some(loot) = maybe_loot {
                    map.world().run(|vm_unit: ViewMut<Unit>| {
                        vm_unit[session.player_entity_id().unwrap()]
                            .set_unit_flag(UnitFlags::Looting);
                    });

                    let packet = ServerMessage::new(SmsgLootResponse::build(
                        &target_guid,
                        LootType::Corpse,
                        loot.money(),
                    ));
                    session.send(&packet).unwrap();
                }
            }
        }
    }
}
