use std::{collections::HashSet, sync::Arc};

use parking_lot::RwLock;
use shared::models::terrain_info::Vector3;
use shipyard::{EntityId, Get, View, ViewMut};

use crate::{
    create_wrapped_resource,
    ecs::components::{
        behavior::Behavior, guid::Guid, movement::Movement, nearby_players::NearbyPlayers,
        unwind::Unwind,
    },
    entities::{
        creature::Creature,
        game_object::GameObject,
        object_guid::ObjectGuid,
        player::Player,
        position::{Position, WorldPosition},
    },
    protocol::packets::SmsgCreateObject,
    session::world_session::WorldSession,
    SessionHolder,
};

use super::{entity_manager::EntityManager, quad_tree::QuadTree, world_context::WorldContext};

pub struct SpatialGrid {
    entities_tree: RwLock<QuadTree>,
    session_holder: Arc<SessionHolder<ObjectGuid>>,
    entity_manager: Arc<EntityManager>,
    world_context: Arc<WorldContext>,
    visibility_distance: f32,
}

impl SpatialGrid {
    pub fn new(
        session_holder: Arc<SessionHolder<ObjectGuid>>,
        entity_manager: Arc<EntityManager>,
        world_context: Arc<WorldContext>,
        visibility_distance: f32,
    ) -> Self {
        Self {
            entities_tree: RwLock::new(QuadTree::new(
                super::quad_tree::QUADTREE_DEFAULT_NODE_CAPACITY,
            )),
            session_holder,
            entity_manager,
            world_context,
            visibility_distance,
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

    pub fn update_entity_position(
        &self,
        entity_guid: &ObjectGuid,
        mover_entity_id: EntityId,
        origin_session: Option<Arc<WorldSession>>, // Must be defined if entity is a player
        new_position: &Position,                   // FIXME: remove the &
        v_movement: &View<Movement>,
        v_player: &View<Player>,
        v_creature: &View<Creature>,
        v_game_object: &View<GameObject>,
        v_guid: &View<Guid>,
        vm_wpos: &mut ViewMut<WorldPosition>,
        vm_behavior: &mut ViewMut<Behavior>,
        vm_nearby_players: &mut ViewMut<NearbyPlayers>,
        vm_unwind: &mut ViewMut<Unwind>,
    ) {
        let is_moving_entity_a_player = origin_session.is_some();
        let previous_position = match self.update(new_position, &mover_entity_id) {
            Some(prev_pos) if !prev_pos.is_same_spot(new_position) => prev_pos,
            _ => return,
        };

        vm_wpos[mover_entity_id].update_local(new_position);

        let visibility_changes = self.get_visibility_changes_for_entities(
            previous_position,
            *new_position,
            mover_entity_id,
        );

        self.handle_entity_appearances(
            entity_guid,
            mover_entity_id,
            &origin_session,
            new_position,
            is_moving_entity_a_player,
            &visibility_changes.moving_entity_appeared_for,
            v_movement,
            v_player,
            v_creature,
            v_game_object,
            v_guid,
            vm_wpos,
            vm_nearby_players,
        );

        self.handle_entity_disappearances(
            entity_guid,
            mover_entity_id,
            &origin_session,
            is_moving_entity_a_player,
            &visibility_changes.moving_entity_disappeared_for,
            v_player,
            v_guid,
            vm_nearby_players,
            vm_unwind,
        );

        self.update_neighbor_behaviors(
            mover_entity_id,
            vm_behavior,
            &visibility_changes.entities_in_range_now,
        );
    }

    fn get_visibility_changes_for_entities(
        &self,
        previous_position: Position,
        new_position: Position,
        mover_entity_id: EntityId,
    ) -> VisibilityChangesAfterMovement {
        let visibility_distance = self.visibility_distance;
        let in_range_before: Vec<EntityId>;
        let in_range_now: Vec<EntityId>;

        let traveled_distance = previous_position.distance_to(new_position, true);
        if traveled_distance <= visibility_distance {
            // Most of the moves are going to be small (entity just walking around), so in these
            // cases we can perform a single search in the quadtree, in a circle that will include
            // both the old and new positions circles (which are going to mostly overlap)
            let center = previous_position.center_between(new_position);
            let search_radius = visibility_distance + traveled_distance / 2.;
            let in_range_all =
                self.search_around_position(&center, search_radius, true, Some(&mover_entity_id));

            let previous_vec3 = previous_position.vec3();
            let new_vec3 = new_position.vec3();
            in_range_before = in_range_all
                .iter()
                .filter_map(|(entity_id, vec3)| {
                    if vec3.square_distance_3d(&previous_vec3)
                        <= visibility_distance * visibility_distance
                    {
                        Some(*entity_id)
                    } else {
                        None
                    }
                })
                .collect();

            in_range_now = in_range_all
                .iter()
                .filter_map(|(entity_id, vec3)| {
                    if vec3.square_distance_3d(&new_vec3)
                        <= visibility_distance * visibility_distance
                    {
                        Some(*entity_id)
                    } else {
                        None
                    }
                })
                .collect();
        } else {
            in_range_before = self.search_ids_around_position(
                &previous_position,
                visibility_distance,
                true,
                Some(&mover_entity_id),
            );
            in_range_now = self
                .search_around_position(
                    &new_position,
                    visibility_distance,
                    true,
                    Some(&mover_entity_id),
                )
                .into_iter()
                .map(|(entity_id, _)| entity_id)
                .collect();
        }

        let in_range_now: HashSet<EntityId> = in_range_now.into_iter().collect();
        let in_range_before: HashSet<EntityId> = in_range_before.into_iter().collect();

        let appeared_for = &in_range_now - &in_range_before;
        let disappeared_for = &in_range_before - &in_range_now;

        VisibilityChangesAfterMovement {
            moving_entity_appeared_for: appeared_for,
            moving_entity_disappeared_for: disappeared_for,
            entities_in_range_before: in_range_before,
            entities_in_range_now: in_range_now,
        }
    }

    fn handle_entity_appearances(
        &self,
        entity_guid: &ObjectGuid,
        mover_entity_id: EntityId,
        origin_session: &Option<Arc<WorldSession>>, // Must be defined if entity is a player
        new_position: &Position,                    // FIXME: remove the &
        is_moving_entity_a_player: bool,
        appeared_for: &HashSet<EntityId>,
        v_movement: &View<Movement>,
        v_player: &View<Player>,
        v_creature: &View<Creature>,
        v_game_object: &View<GameObject>,
        v_guid: &View<Guid>,
        vm_wpos: &mut ViewMut<WorldPosition>,
        vm_nearby_players: &mut ViewMut<NearbyPlayers>,
    ) {
        let mut moving_entity_smsg_create_object: Option<SmsgCreateObject> = None;

        for &other_entity_id in appeared_for {
            let other_guid = v_guid[other_entity_id].0;
            // Make the moving entity appear for the other player
            if let Some(other_session) = self.session_holder.get_session(&other_guid) {
                // Construct the SMSG the first time that it's needed
                if moving_entity_smsg_create_object.is_none() {
                    let movement = v_movement
                        .get(mover_entity_id)
                        .ok()
                        .map(|m| m.build_update(self.world_context.clone(), new_position));

                    if let Ok(player) = v_player.get(mover_entity_id) {
                        moving_entity_smsg_create_object =
                            Some(player.build_create_object(movement, false));
                    } else if let Ok(creature) = v_creature.get(mover_entity_id) {
                        moving_entity_smsg_create_object =
                            Some(creature.build_create_object(movement));
                    }
                }

                other_session.create_entity(
                    entity_guid,
                    moving_entity_smsg_create_object.as_ref().unwrap().clone(),
                );
            }

            // Make the entity (player or otherwise) appear for the moving player
            if let Some(origin_session) = origin_session.as_ref() {
                let smsg_create_object: SmsgCreateObject = {
                    let movement = v_movement.get(other_entity_id).ok().map(|m| {
                        m.build_update(
                            self.world_context.clone(),
                            &vm_wpos[other_entity_id].as_position(),
                        )
                    });

                    if let Ok(player) = v_player.get(other_entity_id) {
                        player.build_create_object(movement, false)
                    } else if let Ok(creature) = v_creature.get(other_entity_id) {
                        creature.build_create_object(movement)
                    } else if let Ok(game_object) = v_game_object.get(other_entity_id) {
                        game_object.build_create_object_for(&v_player[mover_entity_id])
                    } else {
                        unreachable!("cannot generate SMSG_CREATE_OBJECT for this entity type");
                    }
                };

                origin_session.create_entity(&other_guid, smsg_create_object);
            }

            // If a player appeared, increment the NearbyPlayers counter for the
            // creature/gameobject
            if is_moving_entity_a_player {
                NearbyPlayers::increment(other_entity_id, vm_nearby_players);
            }
            // If a creature moved within visibility distance of a player, increment the
            // NearbyPlayers counter for the creature/gameobject
            else if v_player.get(other_entity_id).is_ok() {
                NearbyPlayers::increment(mover_entity_id, vm_nearby_players);
            }
        }
    }

    fn handle_entity_disappearances(
        &self,
        entity_guid: &ObjectGuid,
        mover_entity_id: EntityId,
        origin_session: &Option<Arc<WorldSession>>, // Must be defined if entity is a player
        is_moving_entity_a_player: bool,
        disappeared_for: &HashSet<EntityId>,
        v_player: &View<Player>,
        v_guid: &View<Guid>,
        vm_nearby_players: &mut ViewMut<NearbyPlayers>,
        vm_unwind: &mut ViewMut<Unwind>,
    ) {
        for &other_entity_id in disappeared_for {
            let other_guid = v_guid[other_entity_id].0;
            if let Some(other_session) = self.session_holder.get_session(&other_guid) {
                // Destroy the moving player for the other player
                other_session.destroy_entity(entity_guid);
            }

            // Destroy the other entity for the moving player
            if let Some(os) = origin_session.as_ref() {
                os.destroy_entity(&other_guid)
            }

            // If a player moved away, decrement the NearbyPlayers counter for the creature
            if is_moving_entity_a_player {
                NearbyPlayers::decrement(other_entity_id, vm_nearby_players, vm_unwind);
            }
            // If a creature moved away from a player, decrement the NearbyPlayers counter for
            // the creature
            else if v_player.get(other_entity_id).is_ok() {
                NearbyPlayers::decrement(mover_entity_id, vm_nearby_players, vm_unwind);
            }
        }
    }

    fn update_neighbor_behaviors(
        &self,
        mover_entity_id: EntityId,
        vm_behavior: &mut ViewMut<Behavior>,
        in_range_now: &HashSet<EntityId>,
    ) {
        // If a creature is involved (whether it moved or it witnessed another entity moving),
        // inform its behavior tree that a neighbor has moved (for aggro, script, etc)
        for &neighbor in in_range_now.iter() {
            if let Ok(mut source_behavior) = vm_behavior.get(mover_entity_id) {
                source_behavior.neighbor_moved(neighbor);
            }

            if let Ok(mut neighbor_behavior) = vm_behavior.get(neighbor) {
                neighbor_behavior.neighbor_moved(mover_entity_id);
            }
        }
    }
}

create_wrapped_resource!(WrappedSpatialGrid, SpatialGrid);

#[allow(dead_code)]
struct VisibilityChangesAfterMovement {
    pub moving_entity_appeared_for: HashSet<EntityId>,
    pub moving_entity_disappeared_for: HashSet<EntityId>,
    pub entities_in_range_before: HashSet<EntityId>,
    pub entities_in_range_now: HashSet<EntityId>,
}
