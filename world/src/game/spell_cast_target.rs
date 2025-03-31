use std::sync::Arc;

use log::warn;
use shipyard::EntityId;

use crate::entities::{object_guid::ObjectGuid, position::Position};

use super::map::Map;

/**
 * Spell targets as sent by the client in CMSG_CAST_SPELL.
 * It can be a combination of self, unit, corpse, game object, item entities along with
 * source and destination coordinates.
 */
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SpellCastTargets {
    is_initialized: bool,
    // Data from the ClientPacket
    is_target_self: bool, // Is the spell targeting the player itself?
    unit_guid: Option<ObjectGuid>,
    game_object_guid: Option<ObjectGuid>,
    item_guid: Option<ObjectGuid>,
    //    corpse_guid: Option<Corpse>, // FIXME: Corpses NIY
    source_position: Option<Position>,
    destination_position: Option<Position>,
    string_target: Option<String>,
    // Derived data from the server internal state
    unit_entity_id: Option<EntityId>,
    game_object_entity_id: Option<EntityId>,
    // corpse_entity_id: Option<EntityId>,
}

impl SpellCastTargets {
    pub fn new(
        unit_guid: Option<ObjectGuid>,
        game_object_guid: Option<ObjectGuid>,
        item_guid: Option<ObjectGuid>,
        source_position: Option<Position>,
        destination_position: Option<Position>,
        string_target: Option<String>,
    ) -> Self {
        Self {
            is_initialized: false,
            is_target_self: false,
            unit_guid,
            game_object_guid,
            item_guid,
            source_position,
            destination_position,
            string_target,
            unit_entity_id: None,
            game_object_entity_id: None,
        }
    }

    pub fn new_self() -> Self {
        Self {
            is_initialized: false,
            is_target_self: true,
            unit_guid: None,
            game_object_guid: None,
            item_guid: None,
            source_position: None,
            destination_position: None,
            string_target: None,
            unit_entity_id: None,
            game_object_entity_id: None,
        }
    }

    pub fn new_unit(unit_guid: ObjectGuid) -> Self {
        Self::new(Some(unit_guid), None, None, None, None, None)
    }

    pub fn update_internal_refs(&mut self, map: Arc<Map>) {
        self.is_initialized = true;

        if self.is_target_self {
            self.unit_entity_id = self
                .unit_guid
                .and_then(|unit_guid| map.lookup_entity_ecs(&unit_guid));

            if self.unit_entity_id.is_none() {
                warn!("SpellCastTargets::update_internal_refs - self target unit_guid not found on map");
            }

            return;
        }

        if let Some(other_unit_target_guid) = self.unit_guid {
            self.unit_entity_id = map.lookup_entity_ecs(&other_unit_target_guid);

            if self.unit_entity_id.is_none() {
                warn!("SpellCastTargets::update_internal_refs - other unit_guid not found on map");
            }
        }

        if let Some(game_object_guid) = self.game_object_guid {
            self.game_object_entity_id = map.lookup_entity_ecs(&game_object_guid);

            if self.game_object_entity_id.is_none() {
                warn!("SpellCastTargets::update_internal_refs - game_object_guid not found on map");
            }
        }
    }

    pub fn unit_target(&self) -> Option<EntityId> {
        assert!(
            self.is_initialized,
            "call update_internal_refs on this SpellCastTargets first"
        );

        self.unit_entity_id
    }

    pub fn game_object_target(&self) -> Option<EntityId> {
        assert!(
            self.is_initialized,
            "call update_internal_refs on this SpellCastTargets first"
        );

        self.game_object_entity_id
    }
}
