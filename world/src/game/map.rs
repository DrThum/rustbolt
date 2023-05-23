use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use log::{error, warn};
use shared::models::terrain_info::{TerrainBlock, BLOCK_WIDTH, MAP_WIDTH_IN_BLOCKS};
use tokio::sync::RwLock;

use crate::{
    entities::{
        creature::Creature,
        object_guid::ObjectGuid,
        position::{Position, WorldPosition},
        update::{CreateData, UpdatableEntity},
    },
    protocol::packets::SmsgCreateObject,
    session::world_session::WorldSession,
};

use super::{
    map_manager::{MapKey, TerrainBlockCoords},
    quad_tree::QuadTree,
    world_context::WorldContext,
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
        &self,
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        player_position: &WorldPosition,
        player_guid: &ObjectGuid,
    ) {
        session.send_initial_spells().await;

        let mut guard = self.sessions.write().await;
        if let Some(previous_session) = guard.insert(player_guid.clone(), session.clone()) {
            warn!(
                "session from account {} was already on map {}",
                previous_session.account_id, self.key
            );
        }
        drop(guard);

        {
            let mut tree = self.entities_tree.write().await;
            tree.insert(player_position.to_position(), player_guid.clone());
        }

        {
            // Send the player to themselves
            let player = session.player.read().await;
            let update_data = player.get_create_data(player.guid().raw(), world_context.clone());
            let smsg_update_object = SmsgCreateObject {
                updates_count: update_data.len() as u32,
                has_transport: false,
                updates: update_data,
            };

            session.create_entity(player_guid, smsg_update_object).await;

            for other_session in self
                .sessions_nearby_position(
                    &player_position.to_position(),
                    self.visibility_distance(),
                    true,
                    Some(player_guid),
                )
                .await
            {
                // Broadcast the new player to nearby players
                let other_player = other_session.player.read().await;
                let update_data =
                    player.get_create_data(other_player.guid().raw(), world_context.clone());
                let smsg_update_object = SmsgCreateObject {
                    updates_count: update_data.len() as u32,
                    has_transport: false,
                    updates: update_data,
                };

                other_session
                    .create_entity(player.guid(), smsg_update_object)
                    .await;

                // Send nearby players to the new player
                let update_data =
                    other_player.get_create_data(player.guid().raw(), world_context.clone());
                let smsg_update_object = SmsgCreateObject {
                    updates_count: update_data.len() as u32,
                    has_transport: false,
                    updates: update_data,
                };

                session
                    .create_entity(other_player.guid(), smsg_update_object)
                    .await;
            }
        }
    }

    pub async fn remove_player(&self, session: Arc<WorldSession>) {
        let player_guard = session.player.read().await;
        let player_guid = player_guard.guid();

        {
            let other_sessions = self
                .sessions_nearby_entity(player_guid, self.visibility_distance(), false, false)
                .await;
            for other_session in other_sessions {
                if other_session.is_guid_known(player_guid).await {
                    other_session.destroy_entity(player_guid).await;
                }
            }

            let mut tree = self.entities_tree.write().await;
            tree.delete(player_guid);
        }

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

    pub async fn add_creature(
        &self,
        world_context: Arc<WorldContext>,
        creature: Arc<RwLock<Creature>>,
    ) {
        let creature_guard = creature.read().await;
        let position = creature_guard.position().to_position();
        let guid = creature_guard.guid().clone();

        {
            let mut tree = self.entities_tree.write().await;
            tree.insert(position, guid);
        }

        for session in self
            .sessions_nearby_position(&position, self.visibility_distance(), true, None)
            .await
        {
            // Broadcast the new creature to nearby players
            let player = session.player.read().await;
            let update_data =
                creature_guard.get_create_data(player.guid().raw(), world_context.clone());
            let smsg_update_object = SmsgCreateObject {
                updates_count: update_data.len() as u32,
                has_transport: false,
                updates: update_data,
            };

            session
                .create_entity(player.guid(), smsg_update_object)
                .await;
        }
    }

    pub async fn update_player_position(
        &mut self,
        player_guid: &ObjectGuid,
        origin_session: Arc<WorldSession>,
        new_position: &Position,
        create_data: Vec<CreateData>,
        world_context: Arc<WorldContext>,
    ) {
        let mut tree = self.entities_tree.write().await;

        let previous_position = tree.update(new_position, player_guid);
        drop(tree);

        if let Some(previous_position) = previous_position {
            if previous_position.x == new_position.x
                && previous_position.y == new_position.y
                && previous_position.z == new_position.z
            {
                return;
            }

            let visibility_distance = self.visibility_distance();
            let in_range_before = self
                .sessions_nearby_position(
                    &previous_position,
                    visibility_distance,
                    true,
                    Some(player_guid),
                )
                .await;
            let in_range_before: HashSet<Arc<WorldSession>> =
                in_range_before.iter().cloned().collect();
            let in_range_now = self
                .sessions_nearby_position(
                    new_position,
                    self.visibility_distance(),
                    true,
                    Some(player_guid),
                )
                .await;
            let in_range_now: HashSet<Arc<WorldSession>> = in_range_now.iter().cloned().collect();

            let appeared_for = &in_range_now - &in_range_before;
            let disappeared_for = &in_range_before - &in_range_now;

            let smsg_create_object = SmsgCreateObject {
                updates_count: create_data.len() as u32,
                has_transport: false,
                updates: create_data,
            };

            for other_session in appeared_for {
                // Make the moving player appear for the other player
                other_session
                    .create_entity(player_guid, smsg_create_object.clone())
                    .await;

                // Make the other player appear for the moving player
                let other_player = other_session.player.read().await;
                let create_data =
                    other_player.get_create_data(player_guid.raw(), world_context.clone());
                let smsg_create_object = SmsgCreateObject {
                    updates_count: create_data.len() as u32,
                    has_transport: false,
                    updates: create_data,
                };
                origin_session
                    .create_entity(other_player.guid(), smsg_create_object)
                    .await;
            }

            for other_session in disappeared_for {
                // Destroy the moving player for the other player
                other_session.destroy_entity(player_guid).await;

                // Destroy the other player for the moving player
                let other_player = other_session.player.read().await;
                let other_player_guid = other_player.guid();
                origin_session.destroy_entity(other_player_guid).await;
            }
        } else {
            error!("updating position for player not on map");
        }
    }

    pub async fn sessions_nearby_entity(
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

    pub async fn sessions_nearby_position(
        &self,
        position: &Position,
        range: f32,
        search_in_3d: bool,
        exclude_guid: Option<&ObjectGuid>,
    ) -> Vec<Arc<WorldSession>> {
        let guids =
            self.entities_tree
                .read()
                .await
                .search_around_position(position, range, search_in_3d);

        self.sessions
            .read()
            .await
            .iter()
            .filter_map(|(&guid, session)| {
                let excluded = exclude_guid.map_or(false, |&ex| ex == guid);

                if guids.contains(&guid) && !excluded {
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
