use std::sync::Arc;

use shipyard::ViewMut;

use crate::ecs::components::unit::Unit;
use crate::entities::object_guid::ObjectGuid;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_realm_split(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
        _vm_all_storages: Option<AllStoragesViewMut>,
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
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
        _vm_all_storages: Option<AllStoragesViewMut>,
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
}
