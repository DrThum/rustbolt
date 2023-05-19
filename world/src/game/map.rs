use std::{collections::HashMap, sync::Arc};

use log::warn;
use shared::models::terrain_info::{TerrainBlock, BLOCK_WIDTH, MAP_WIDTH_IN_BLOCKS};
use tokio::sync::RwLock;

use crate::{
    entities::{
        object_guid::ObjectGuid,
        position::{Position, WorldPosition},
    },
    session::world_session::WorldSession,
};

use super::{
    map_manager::{MapKey, TerrainBlockCoords},
    quad_tree::QuadTree,
};

pub const DEFAULT_VISIBILITY_DISTANCE: f32 = 90.0;

pub struct Map {
    key: MapKey,
    sessions: RwLock<HashMap<ObjectGuid, Arc<WorldSession>>>,
    terrain: Arc<HashMap<TerrainBlockCoords, TerrainBlock>>,
    entities_tree: RwLock<QuadTree>,
    visibility_distance: f32,
}

impl Map {
    pub fn new(key: MapKey, terrain: Arc<HashMap<TerrainBlockCoords, TerrainBlock>>) -> Self {
        Self {
            key,
            sessions: RwLock::new(HashMap::new()),
            terrain,
            entities_tree: RwLock::new(QuadTree::new(
                super::quad_tree::QUADTREE_DEFAULT_NODE_CAPACITY,
            )),
            visibility_distance: DEFAULT_VISIBILITY_DISTANCE,
        }
    }

    pub async fn add_player(
        &mut self,
        session: Arc<WorldSession>,
        player_position: &WorldPosition,
        player_guid: &ObjectGuid,
    ) {
        session.send_initial_spells().await;

        let mut guard = self.sessions.write().await;
        if let Some(previous_session) = guard.insert(player_guid.clone(), session) {
            warn!(
                "session from account {} was already on map {}",
                previous_session.account_id, self.key
            );
        }

        let mut tree = self.entities_tree.write().await;
        tree.insert(player_position.to_position(), player_guid.clone());
    }

    pub async fn remove_player(&mut self, session: Arc<WorldSession>) {
        let player_guard = session.player.read().await;
        let player_guid = player_guard.guid();
        let mut tree = self.entities_tree.write().await;
        tree.delete(player_guid);

        {
            let mut guard = self.sessions.write().await;
            if let None = guard.remove(player_guid) {
                warn!(
                    "session from account {} was not on map {}",
                    session.account_id, self.key
                );
            }
        }
    }

    pub async fn update_player_position(&mut self, player_guid: &ObjectGuid, position: &Position) {
        self.entities_tree
            .write()
            .await
            .update(position, player_guid);
    }

    pub async fn nearby_sessions(
        &self,
        source_guid: &ObjectGuid,
        range: f32,
        search_in_3d: bool,
        include_self: bool,
    ) -> Vec<Arc<WorldSession>> {
        let mut guids =
            self.entities_tree
                .read()
                .await
                .search_around_entity(source_guid, range, search_in_3d);

        if !include_self {
            guids.retain(|g| g != source_guid);
        }

        self.sessions
            .read()
            .await
            .iter()
            .filter_map(|(&guid, session)| {
                if guids.contains(&guid) {
                    Some(session.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_terrain_height(&self, position_x: f32, position_y: f32) -> Option<f32> {
        let offset: f32 = MAP_WIDTH_IN_BLOCKS as f32 / 2.0;
        let block_row = (offset - (position_x / BLOCK_WIDTH)).floor() as usize;
        let block_col = (offset - (position_y / BLOCK_WIDTH)).floor() as usize;
        let terrain_block_coords = TerrainBlockCoords {
            row: block_row,
            col: block_col,
        };
        if let Some(terrain_block) = self.terrain.get(&terrain_block_coords) {
            return Some(terrain_block.get_height(position_x, position_y));
            // TODO: terrain_block.map(_.get_height) instead
        }

        let position_z = 0.0;
        Some(position_z)
    }

    pub fn visibility_distance(&self) -> f32 {
        self.visibility_distance
    }
}
