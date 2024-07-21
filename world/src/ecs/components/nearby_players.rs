use std::sync::atomic::{AtomicUsize, Ordering};

use log::warn;
use shipyard::{AddComponent, Component, Delete, EntityId, Get, ViewMut};

use super::unwind::Unwind;

#[derive(Component)]
pub struct NearbyPlayers {
    pub count: AtomicUsize,
}

impl Default for NearbyPlayers {
    fn default() -> Self {
        Self::new()
    }
}

impl NearbyPlayers {
    pub fn new() -> Self {
        Self {
            count: AtomicUsize::new(1),
        }
    }

    // Increment the count of nearby players, adding the NearbyPlayers component if it wasn't set
    pub fn increment(entity_id: EntityId, vm_nearby_players: &mut ViewMut<NearbyPlayers>) {
        if let Ok(nearby_players) = vm_nearby_players.get(entity_id) {
            nearby_players.count.fetch_add(1, Ordering::Relaxed);
        } else {
            vm_nearby_players.add_component_unchecked(entity_id, NearbyPlayers::new());
        }
    }

    // Decrement the count of nearby players
    // If the count reaches 0, remove the NearbyPlayers component and mark the creature as
    // "Unwinding", giving it some time to properly reset its state (movement, combat, ...)
    pub fn decrement(
        entity_id: EntityId,
        vm_nearby_players: &mut ViewMut<NearbyPlayers>,
        vm_unwind: &mut ViewMut<Unwind>,
    ) {
        if let Ok(nearby_players) = vm_nearby_players.get(entity_id) {
            let previous_value = nearby_players.count.fetch_sub(1, Ordering::Relaxed);
            if previous_value <= 1 {
                vm_nearby_players.delete(entity_id);
                vm_unwind.add_component_unchecked(entity_id, Unwind::default());
            }
        } else {
            warn!(
                "NearbyPlayers::decrement called for an entity that has no NearbyPlayers component"
            );
        }
    }
}
