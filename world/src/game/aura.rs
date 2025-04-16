use std::time::{Duration, Instant};

use shipyard::EntityId;

use crate::entities::object_guid::ObjectGuid;

pub struct Aura {
    pub spell_id: u32,
    pub spell_effect_index: usize,
    pub caster_id: EntityId,
    pub caster_guid: ObjectGuid,
    pub target_id: EntityId,
    pub target_guid: ObjectGuid,
    pub is_positive: bool,
    pub expires: Instant,
}

impl Aura {
    pub fn new(
        spell_id: u32,
        spell_effect_index: usize,
        caster_id: EntityId,
        caster_guid: ObjectGuid,
        target_id: EntityId,
        target_guid: ObjectGuid,
        duration: Duration,
    ) -> Self {
        Self {
            spell_id,
            spell_effect_index,
            caster_id,
            caster_guid,
            target_id,
            target_guid,
            is_positive: true, // FIXME
            expires: Instant::now() + duration,
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

    pub fn is_expired(&self, now: Instant) -> bool {
        self.expires <= now
    }
}
