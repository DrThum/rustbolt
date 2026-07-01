use shared::models::terrain_info::Vector3;
use shipyard::EntityId;

use crate::{entities::position::Position, game::quad_tree::QuadTree};

pub trait SpatialIndex {
    fn insert(&mut self, pos: Position, entity_id: EntityId);

    fn update(&mut self, new_position: &Position, entity_id: &EntityId) -> Option<Position>;

    fn delete(&mut self, guid: &EntityId) -> Option<Position>;

    fn search_around_position(
        &self,
        position: &Position,
        radius: f32,
        search_in_3d: bool,
        exclude_id: Option<&EntityId>,
    ) -> Vec<(EntityId, Vector3)>;

    fn search_around_entity(
        &self,
        entity_id: &EntityId,
        radius: f32,
        search_in_3d: bool,
        exclude_id: Option<&EntityId>,
    ) -> Vec<(EntityId, Vector3)>;
}

pub enum SpatialBackend {
    Quad(QuadTree),
    // Grid(CellGrid),
}

impl SpatialIndex for SpatialBackend {
    fn insert(&mut self, pos: Position, entity_id: EntityId) {
        match self {
            SpatialBackend::Quad(quad_tree) => quad_tree.insert(pos, entity_id),
        }
    }

    fn update(&mut self, new_position: &Position, entity_id: &EntityId) -> Option<Position> {
        match self {
            SpatialBackend::Quad(quad_tree) => quad_tree.update(new_position, entity_id),
        }
    }

    fn delete(&mut self, guid: &EntityId) -> Option<Position> {
        match self {
            SpatialBackend::Quad(quad_tree) => quad_tree.delete(guid),
        }
    }

    fn search_around_position(
        &self,
        position: &Position,
        radius: f32,
        search_in_3d: bool,
        exclude_id: Option<&EntityId>,
    ) -> Vec<(EntityId, Vector3)> {
        match self {
            SpatialBackend::Quad(quad_tree) => {
                quad_tree.search_around_position(position, radius, search_in_3d, exclude_id)
            }
        }
    }

    fn search_around_entity(
        &self,
        entity_id: &EntityId,
        radius: f32,
        search_in_3d: bool,
        exclude_id: Option<&EntityId>,
    ) -> Vec<(EntityId, Vector3)> {
        match self {
            SpatialBackend::Quad(quad_tree) => {
                quad_tree.search_around_entity(entity_id, radius, search_in_3d, exclude_id)
            }
        }
    }
}
