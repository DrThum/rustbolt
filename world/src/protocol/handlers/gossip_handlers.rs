use log::{error, warn};
use shipyard::{Get, View};

use crate::entities::creature::Creature;
use crate::game::gossip::GossipMenu;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};
use crate::shared::constants::GossipMenuOptionType;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_gossip_hello(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgGossipHello = ClientMessage::read_as(data).unwrap();

        OpcodeHandler::send_initial_gossip_menu(cmsg.guid, session.clone(), world_context.clone());
    }

    pub fn handle_cmsg_gossip_select_option(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgGossipSelectOption = ClientMessage::read_as(data).unwrap();

        let Some(map) = session.current_map() else {
            error!("handle_cmsg_gossip_select_option: session has no map");
            return;
        };

        let Some(target_entity_id) = map.lookup_entity_ecs(&cmsg.guid) else {
            error!(
                "handle_cmsg_gossip_select_option: map has no EntityId for cmsg.guid (guid: {:?})",
                cmsg.guid
            );
            return;
        };

        let creature_exists = map
            .world()
            .run(|v_creature: View<Creature>| v_creature.get(target_entity_id).is_ok());
        if !creature_exists {
            warn!("handle_cmsg_gossip_select_option: target is not a creature, TODO!");
            return;
        };

        let Some(gossip_menu_record) = world_context.data_store.get_gossip_menu(cmsg.menu_id)
        else {
            error!("handle_cmsg_gossip_select_option: received a non-existing menu_id");
            return;
        };

        let gossip_menu = GossipMenu::from_db_record(gossip_menu_record);

        if cmsg.option_index as usize >= gossip_menu.items.len() {
            error!("handle_cmsg_gossip_select_option: received a non-existing option_id (index {} but menu only has {} items)", cmsg.option_index, gossip_menu.items.len());
            return;
        }

        let gossip_menu_option = &gossip_menu_record.options[cmsg.option_index as usize];

        match gossip_menu_option.option_type {
            GossipMenuOptionType::Innkeeper => {
                println!("TODO: update player bind point");
            },
            ot => warn!("handle_cmsg_gossip_select_option: received a non-implemented-yet option type {ot:?}"),
        };
    }
}
