use std::sync::atomic::{AtomicUsize, Ordering};

use log::warn;
use shipyard::{AddComponent, Component, Delete, EntityId, Get, ViewMut};

#[derive(Component)]
pub struct NearbyPlayers {
    pub count: AtomicUsize,
}

impl NearbyPlayers {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(1),
        }
    }

    pub fn increment(entity_id: EntityId, vm_nearby_players: &mut ViewMut<NearbyPlayers>) {
        if let Ok(nearby_players) = vm_nearby_players.get(entity_id) {
            nearby_players.count.fetch_add(1, Ordering::Relaxed);
        } else {
            vm_nearby_players.add_component_unchecked(entity_id, NearbyPlayers::new());
        }
    }

    pub fn decrement(entity_id: EntityId, vm_nearby_players: &mut ViewMut<NearbyPlayers>) {
        if let Ok(nearby_players) = vm_nearby_players.get(entity_id) {
            let previous_value = nearby_players.count.fetch_sub(1, Ordering::Relaxed);
            if previous_value <= 1 {
                vm_nearby_players.delete(entity_id);
            }
        } else {
            warn!(
                "NearbyPlayers::decrement called for an entity that has no NearbyPlayers component"
            );
        }
    }
}
