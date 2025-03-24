use std::sync::Arc;

use crate::{
    create_wrapped_resource,
    entities::object_guid::ObjectGuid,
    protocol::{self, server::ServerMessage},
};

use super::{entity_manager::EntityManager, spatial_grid::SpatialGrid};

pub struct PacketBroadcaster {
    spatial_grid: Arc<SpatialGrid>,
    entity_manager: Arc<EntityManager>,
    visibility_distance: f32,
}

impl PacketBroadcaster {
    pub fn new(
        spatial_grid: Arc<SpatialGrid>,
        entity_manager: Arc<EntityManager>,
        visibility_distance: f32,
    ) -> Self {
        Self {
            spatial_grid,
            entity_manager,
            visibility_distance,
        }
    }

    pub fn broadcast_packet<
        const OPCODE: u16,
        Payload: protocol::server::ServerMessagePayload<OPCODE>,
    >(
        &self,
        origin_guid: &ObjectGuid,
        packet: &ServerMessage<OPCODE, Payload>,
        range: Option<f32>,
        include_self: bool,
    ) {
        if let Some(origin_entity_id) = self.entity_manager.lookup(origin_guid) {
            for session in self.spatial_grid.sessions_nearby_entity(
                &origin_entity_id,
                range.unwrap_or(self.visibility_distance),
                true,
                include_self,
            ) {
                session.send(packet).unwrap();
            }
        }
    }
}

create_wrapped_resource!(WrappedPacketBroadcaster, PacketBroadcaster);
