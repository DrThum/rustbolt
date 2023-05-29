use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use log::{error, warn};
use shared::models::terrain_info::{TerrainBlock, BLOCK_WIDTH, MAP_WIDTH_IN_BLOCKS};
use tokio::sync::RwLock;

use crate::{
    entities::{
        creature::Creature,
        object_guid::ObjectGuid,
        player::Player,
        position::Position,
        update::{CreateData, WorldEntity},
    },
    protocol::packets::{SmsgCreateObject, SmsgUpdateObject},
    repositories::creature::CreatureSpawnDbRecord,
    session::world_session::WorldSession,
    DataStore,
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
    entities: RwLock<HashMap<ObjectGuid, Arc<RwLock<dyn WorldEntity + Sync + Send>>>>,
    terrain: Arc<HashMap<TerrainBlockCoords, TerrainBlock>>,
    entities_tree: RwLock<QuadTree>,
    visibility_distance: f32,
}

impl Map {
    pub async fn new(
        key: MapKey,
        terrain: Arc<HashMap<TerrainBlockCoords, TerrainBlock>>,
        spawns: Vec<CreatureSpawnDbRecord>,
        data_store: Arc<DataStore>,
    ) -> Map {
        let map = Map {
            key,
            sessions: RwLock::new(HashMap::new()),
            entities: RwLock::new(HashMap::new()),
            terrain,
            entities_tree: RwLock::new(QuadTree::new(
                super::quad_tree::QUADTREE_DEFAULT_NODE_CAPACITY,
            )),
            visibility_distance: DEFAULT_VISIBILITY_DISTANCE,
        };

        for spawn in spawns {
            if let Some(creature) = Creature::from_spawn(&spawn, data_store.clone()) {
                map.add_creature(None, Arc::new(RwLock::new(creature)))
                    .await;
            } else {
                warn!("failed to spawn creature with guid {}", spawn.guid);
            }
        }

        map
    }

    pub async fn add_player(
        &self,
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        player: Arc<RwLock<Player>>,
    ) {
        let player_guard = player.read().await;
        let player_guid = player_guard.guid().clone();
        let player_position = player_guard.position().to_position();
        drop(player_guard);

        session.send_initial_spells().await;
        session.send_initial_action_buttons().await;
        session.send_initial_reputations().await;

        {
            let mut guard = self.sessions.write().await;
            if let Some(previous_session) = guard.insert(player_guid.clone(), session.clone()) {
                warn!(
                    "session from account {} was already on map {}",
                    previous_session.account_id, self.key
                );
            }
        }

        {
            let mut tree = self.entities_tree.write().await;
            tree.insert(player_position, player_guid);
        }

        self.entities
            .write()
            .await
            .insert(player_guid.clone(), player.clone());

        {
            let player_guard = player.read().await;

            // TODO: Maybe we can group all updates within the same packet?
            for guid in self.entities_tree.read().await.search_around_position(
                &player_position,
                self.visibility_distance(),
                true,
                None,
            ) {
                if let Some(entity) = world_context
                    .map_manager
                    .lookup_entity(&guid, Some(self.key))
                    .await
                {
                    // Broadcast the new player to nearby players and to itself
                    if let Some(other_session) = self.sessions.read().await.get(&guid) {
                        let update_data =
                            player_guard.get_create_data(guid.raw(), world_context.clone());
                        let smsg_update_object = SmsgCreateObject {
                            updates_count: update_data.len() as u32,
                            has_transport: false,
                            updates: update_data,
                        };

                        other_session
                            .create_entity(&player_guid, smsg_update_object)
                            .await;
                    }

                    // Send nearby entities to the new player
                    if guid != player_guid {
                        // Don't send the player to itself twice
                        let update_data = entity
                            .read()
                            .await
                            .get_create_data(player_guid.raw(), world_context.clone());
                        let smsg_update_object = SmsgCreateObject {
                            updates_count: update_data.len() as u32,
                            has_transport: false,
                            updates: update_data,
                        };

                        session.create_entity(&guid, smsg_update_object).await;
                    }
                } else {
                    error!("found an entity in quadtree but not in MapManager");
                }
            }
        }
    }

    pub async fn remove_player(&self, player_guid: &ObjectGuid) {
        self.entities.write().await.remove(player_guid);

        {
            let other_sessions = self
                .sessions_nearby_entity(player_guid, self.visibility_distance(), false, false)
                .await;
            for other_session in other_sessions {
                other_session.destroy_entity(player_guid).await;
            }

            let mut tree = self.entities_tree.write().await;
            tree.delete(player_guid);
        }

        {
            let mut guard = self.sessions.write().await;
            if let None = guard.remove(player_guid) {
                warn!("player guid {:?} was not on map {}", player_guid, self.key);
            }
        }
    }

    pub async fn add_creature(
        &self,
        world_context: Option<Arc<WorldContext>>, // None during startup
        creature: Arc<RwLock<Creature>>,
    ) {
        let creature_guard = creature.read().await;
        let position = creature_guard.position().to_position();
        let guid = creature_guard.guid().clone();

        self.entities.write().await.insert(guid, creature.clone());

        {
            let mut tree = self.entities_tree.write().await;
            tree.insert(position, guid);
        }

        if let Some(world_context) = world_context {
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
            let in_range_before = self.entities_tree.read().await.search_around_position(
                &previous_position,
                visibility_distance,
                true,
                Some(player_guid),
            );
            let in_range_before: HashSet<ObjectGuid> = in_range_before.iter().cloned().collect();
            let in_range_now = self.entities_tree.read().await.search_around_position(
                new_position,
                self.visibility_distance(),
                true,
                Some(player_guid),
            );
            let in_range_now: HashSet<ObjectGuid> = in_range_now.iter().cloned().collect();

            let appeared_for = &in_range_now - &in_range_before;
            let disappeared_for = &in_range_before - &in_range_now;

            let smsg_create_object = SmsgCreateObject {
                updates_count: create_data.len() as u32,
                has_transport: false,
                updates: create_data,
            };

            for other_guid in appeared_for {
                if let Some(entity) = self.lookup_entity(&other_guid).await {
                    if let Some(other_session) = self.sessions.read().await.get(&other_guid) {
                        // Make the moving player appear for the other player
                        other_session
                            .create_entity(player_guid, smsg_create_object.clone())
                            .await;
                    }

                    // Make the entity (player or otherwise) appear for the moving player
                    let create_data = entity
                        .read()
                        .await
                        .get_create_data(player_guid.raw(), world_context.clone());
                    let smsg_create_object = SmsgCreateObject {
                        updates_count: create_data.len() as u32,
                        has_transport: false,
                        updates: create_data,
                    };
                    origin_session
                        .create_entity(&other_guid, smsg_create_object)
                        .await;
                }
            }

            for other_guid in disappeared_for {
                if let Some(other_session) = self.sessions.read().await.get(&other_guid) {
                    // Destroy the moving player for the other player
                    other_session.destroy_entity(player_guid).await;
                }

                // Destroy the other entity for the moving player
                origin_session.destroy_entity(&other_guid).await;
            }
        } else {
            error!("updating position for player not on map");
        }
    }

    pub async fn lookup_entity(
        &self,
        guid: &ObjectGuid,
    ) -> Option<Arc<RwLock<dyn WorldEntity + Sync + Send>>> {
        self.entities.read().await.get(guid).cloned()
    }

    pub async fn sessions_nearby_entity(
        &self,
        source_guid: &ObjectGuid,
        range: f32,
        search_in_3d: bool,
        include_self: bool,
    ) -> Vec<Arc<WorldSession>> {
        let guids = self.entities_tree.read().await.search_around_entity(
            source_guid,
            range,
            search_in_3d,
            if include_self {
                None
            } else {
                Some(source_guid)
            },
        );

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
        let guids = self.entities_tree.read().await.search_around_position(
            position,
            range,
            search_in_3d,
            exclude_guid,
        );

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

    pub async fn tick(&self, diff: Duration, world_context: Arc<WorldContext>) {
        let entities = self.entities.read().await;
        for (_, entity) in &*entities {
            let mut entity = entity.write().await;
            entity.tick(diff, world_context.clone()).await;

            // Broadcast the changes to nearby players
            if entity.has_updates() {
                for session in self
                    .sessions_nearby_entity(entity.guid(), self.visibility_distance(), true, false)
                    .await
                {
                    let update_data = entity.get_update_data(
                        session.player.read().await.guid().raw(),
                        world_context.clone(),
                    );

                    let smsg_update_object = SmsgUpdateObject {
                        updates_count: update_data.len() as u32,
                        has_transport: false,
                        updates: update_data,
                    };

                    session.update_entity(smsg_update_object).await;
                }

                entity.mark_up_to_date();
            }
        }
    }
}
