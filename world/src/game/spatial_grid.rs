use std::sync::Arc;

use parking_lot::RwLock;
use shared::models::terrain_info::Vector3;
use shipyard::{EntityId, Unique};

use crate::{
    entities::{object_guid::ObjectGuid, position::Position},
    session::world_session::WorldSession,
    SessionHolder,
};

use super::{entity_manager::EntityManager, quad_tree::QuadTree};

pub struct SpatialGrid {
    entities_tree: RwLock<QuadTree>,
    session_holder: Arc<SessionHolder<ObjectGuid>>,
    entity_manager: Arc<EntityManager>,
}

impl SpatialGrid {
    pub fn new(
        session_holder: Arc<SessionHolder<ObjectGuid>>,
        entity_manager: Arc<EntityManager>,
    ) -> Self {
        Self {
            entities_tree: RwLock::new(QuadTree::new(
                super::quad_tree::QUADTREE_DEFAULT_NODE_CAPACITY,
            )),
            session_holder,
            entity_manager,
        }
    }

    pub fn insert(&self, position: Position, entity_id: EntityId) {
        self.entities_tree.write().insert(position, entity_id);
    }

    pub fn delete(&self, entity_id: &EntityId) {
        self.entities_tree.write().delete(entity_id);
    }

    pub fn update(&self, position: &Position, entity_id: &EntityId) -> Option<Position> {
        self.entities_tree.write().update(position, entity_id)
    }

    pub fn search_around_position(
        &self,
        position: &Position,
        radius: f32,
        search_in_3d: bool,
        exclude_id: Option<&EntityId>,
    ) -> Vec<(EntityId, Vector3)> {
        self.entities_tree
            .read()
            .search_around_position(position, radius, search_in_3d, exclude_id)
    }

    pub fn search_ids_around_position(
        &self,
        position: &Position,
        radius: f32,
        search_in_3d: bool,
        exclude_id: Option<&EntityId>,
    ) -> Vec<EntityId> {
        self.search_around_position(position, radius, search_in_3d, exclude_id)
            .into_iter()
            .map(|(entity_id, _)| entity_id)
            .collect()
    }

    pub fn search_around_entity(
        &self,
        entity_id: &EntityId,
        radius: f32,
        search_in_3d: bool,
        exclude_id: Option<&EntityId>,
    ) -> Vec<(EntityId, Vector3)> {
        self.entities_tree
            .read()
            .search_around_entity(entity_id, radius, search_in_3d, exclude_id)
    }

    pub fn sessions_nearby_entity(
        &self,
        source_entity_id: &EntityId,
        range: f32,
        search_in_3d: bool,
        include_self: bool,
    ) -> Vec<Arc<WorldSession>> {
        let entity_ids: Vec<EntityId> = self
            .search_around_entity(
                source_entity_id,
                range,
                search_in_3d,
                if include_self {
                    None
                } else {
                    Some(source_entity_id)
                },
            )
            .into_iter()
            .map(|(entity_id, _)| entity_id)
            .collect();

        self.session_holder.get_matching_sessions(|guid| {
            if let Some(entity_id) = self.entity_manager.lookup(guid) {
                return entity_ids.contains(&entity_id);
            }

            false
        })
    }

    pub fn sessions_nearby_position(
        &self,
        position: &Position,
        range: f32,
        search_in_3d: bool,
        exclude_id: Option<&EntityId>,
    ) -> Vec<Arc<WorldSession>> {
        let entity_ids: Vec<EntityId> =
            self.search_ids_around_position(position, range, search_in_3d, exclude_id);

        self.session_holder.get_matching_sessions(|guid| {
            if let Some(entity_id) = self.entity_manager.lookup(guid) {
                return entity_ids.contains(&entity_id);
            }

            false
        })
    }
}

#[derive(Unique)]
pub struct WrappedSpatialGrid(pub Arc<SpatialGrid>);
