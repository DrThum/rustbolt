use shipyard::EntityId;

use crate::entities::object_guid::ObjectGuid;

pub struct Aura {
    pub spell_id: u32,
    pub caster_id: EntityId,
    pub caster_guid: ObjectGuid,
    pub target_id: EntityId,
    pub is_positive: bool,
}

impl Aura {
    pub fn new(
        spell_id: u32,
        caster_id: EntityId,
        caster_guid: ObjectGuid,
        target_id: EntityId,
    ) -> Self {
        Self {
            spell_id,
            caster_id,
            caster_guid,
            target_id,
            is_positive: true, // FIXME
        }
    }

    pub fn is_visible(&self) -> bool {
        // FIXME: !IsPassive() || GetSpellInfo()->HasAreaAuraEffect()
        true
    }

    pub fn level(&self) -> u8 {
        1 // FIXME
    }

    pub fn stack_count(&self) -> u8 {
        0 // FIXME: should return an Option with None for unstackable auras
    }
}
