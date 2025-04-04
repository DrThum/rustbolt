use shipyard::EntityId;

use crate::entities::object_guid::ObjectGuid;

pub struct Spell {
    id: u32,
    cast_from_item_id: Option<u32>,
    caster_entity_id: EntityId,
    caster_guid: ObjectGuid,
    unit_target: Option<EntityId>,
    game_object_target: Option<EntityId>,
    // item_target: EntityId, // TODO: We'll have to make Item a Component for this to work
    power_cost: u32,
}

impl Spell {
    pub fn new(
        id: u32,
        cast_from_item_id: Option<u32>,
        caster_entity_id: EntityId,
        caster_guid: ObjectGuid,
        unit_target: Option<EntityId>,
        game_object_target: Option<EntityId>,
        power_cost: u32,
    ) -> Self {
        Self {
            id,
            cast_from_item_id,
            caster_entity_id,
            caster_guid,
            unit_target,
            game_object_target,
            power_cost,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn cast_from_item_id(&self) -> Option<u32> {
        self.cast_from_item_id
    }

    pub fn unit_target(&self) -> Option<EntityId> {
        self.unit_target
    }

    pub fn game_object_target(&self) -> Option<EntityId> {
        self.game_object_target
    }

    pub fn caster(&self) -> EntityId {
        self.caster_entity_id
    }

    pub fn caster_guid(&self) -> ObjectGuid {
        self.caster_guid
    }

    pub fn power_cost(&self) -> u32 {
        self.power_cost
    }
}
