use parking_lot::RwLock;
use shared::models::terrain_info::Vector3;
use shipyard::EntityId;

use crate::entities::position::Position;

use super::quad_tree::QuadTree;

pub struct SpatialGrid {
    entities_tree: RwLock<QuadTree>,
}

impl SpatialGrid {
    pub fn new() -> Self {
        Self {
            entities_tree: RwLock::new(QuadTree::new(
                super::quad_tree::QUADTREE_DEFAULT_NODE_CAPACITY,
            )),
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
}
