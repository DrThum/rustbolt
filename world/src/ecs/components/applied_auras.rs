use std::sync::Arc;

use enumflags2::{make_bitflags, BitFlags};
use fixedbitset::FixedBitSet;
use log::warn;
use parking_lot::RwLock;
use shipyard::Component;

use crate::{
    datastore::data_types::SpellRecord,
    entities::{internal_values::InternalValues, update_fields::UnitFields},
    game::{aura::Aura, spell::Spell},
    protocol::{
        packets::{SmsgSetExtraAuraInfo, SmsgSetExtraAuraInfoNeedUpdate, SmsgUpdateAuraDuration},
        server::ServerMessage,
    },
    session::world_session::WorldSession,
    shared::constants::{AuraFlag, UNIT_AURAS_LIMIT},
    DataStore,
};

#[derive(Component)]
pub struct AppliedAuras {
    auras: Vec<AuraApplication>,
    visible_positive_aura_slots_occupation: FixedBitSet, // Bit set to 1 = slot is occupied
    visible_negative_aura_slots_occupation: FixedBitSet, // Bit set to 1 = slot is occupied
    internal_values: Arc<RwLock<InternalValues>>,
}

impl AppliedAuras {
    pub fn new(max_positive_auras: usize, internal_values: Arc<RwLock<InternalValues>>) -> Self {
        {
            let mut values = internal_values.write();
            // Reset all auras related internal
            let start = UnitFields::UnitFieldAura as usize;
            let end = UnitFields::UnitFieldAuraState as usize;

            (start..end).for_each(|index| {
                values.set_u32(index, 0);
            });
        }

        Self {
            auras: Vec::new(),
            visible_positive_aura_slots_occupation: FixedBitSet::with_capacity(max_positive_auras),
            visible_negative_aura_slots_occupation: FixedBitSet::with_capacity(
                UNIT_AURAS_LIMIT - max_positive_auras,
            ),
            internal_values,
        }
    }

    pub fn add_aura(
        &mut self,
        spell: Arc<Spell>,
        spell_record: Arc<SpellRecord>,
        caster_session: Option<Arc<WorldSession>>,
        target_session: Option<Arc<WorldSession>>,
        data_store: Arc<DataStore>,
    ) {
        match self.auras.iter().find(|aura| aura.spell_id() == spell.id()) {
            Some(_existing_aura) => {
                warn!("not implemented: refresh aura");
            }
            None => {
                let Some(target_entity_id) = spell.unit_target() else {
                    warn!("add_aura: spell has no unit target (TODO?)");
                    return;
                };

                let Some(target_guid) = spell.unit_target_guid() else {
                    warn!("add_aura: spell has no unit target guid (TODO?)");
                    return;
                };

                let aura = Aura::new(
                    spell.id(),
                    spell.caster(),
                    spell.caster_guid(),
                    target_entity_id,
                );

                let mut slot: Option<usize> = None;
                if aura.is_visible() {
                    if let Some(first_free_slot) = self.find_first_free_slot(aura.is_positive) {
                        slot = Some(first_free_slot);

                        // Update internal values for this specific slot
                        let mut values = self.internal_values.write();

                        values.set_u32(
                            UnitFields::UnitFieldAura as usize + first_free_slot,
                            aura.spell_id,
                        );

                        let update_field_slot = first_free_slot / 4;
                        let update_field_offset = first_free_slot % 4;

                        let aura_flags = if aura.is_positive {
                            make_bitflags!(AuraFlag::{Helpful}).bits()
                        } else {
                            BitFlags::from_flag(AuraFlag::Harmful).bits()
                        };

                        values.set_u8(
                            UnitFields::UnitFieldAuraFlags as usize + update_field_slot,
                            update_field_offset,
                            aura_flags,
                        );

                        values.set_u8(
                            UnitFields::UnitFieldAuraLevels as usize + update_field_slot,
                            update_field_offset,
                            aura.level(),
                        );

                        values.set_u8(
                            UnitFields::UnitFieldAuraApplications as usize + update_field_slot,
                            update_field_offset,
                            aura.stack_count(),
                        );
                    } else {
                        warn!("unable to find a slot for the new aura, TODO!");
                        return;
                    }
                }

                let duration = spell_record
                    .base_duration(data_store.clone())
                    .map(|d| d.as_millis())
                    .unwrap_or(0) as u32;

                if let Some(slot) = slot {
                    if let Some(session) = target_session {
                        let packet = ServerMessage::new(SmsgUpdateAuraDuration {
                            slot: slot as u8,
                            duration_ms: duration,
                        });

                        session.send(&packet).unwrap();

                        let packet = ServerMessage::new(SmsgSetExtraAuraInfo {
                            target_guid: target_guid.as_packed(),
                            slot: slot as u8,
                            spell_id: aura.spell_id,
                            max_duration_ms: duration,
                            duration_ms: duration,
                        });

                        session.send(&packet).unwrap();
                    }

                    if let Some(caster_session) = caster_session {
                        if spell.caster() != target_entity_id {
                            let packet = ServerMessage::new(SmsgSetExtraAuraInfoNeedUpdate {
                                target_guid: target_guid.as_packed(),
                                slot: slot as u8,
                                spell_id: aura.spell_id,
                                max_duration_ms: duration,
                                duration_ms: duration,
                            });

                            caster_session.send(&packet).unwrap();
                        }
                    }

                    self.lock_slot(slot, aura.is_positive);
                }

                let aura_app = AuraApplication::new(aura, slot);
                self.auras.push(aura_app);
            }
        }
    }

    fn find_first_free_slot(&self, is_positive_aura: bool) -> Option<usize> {
        let bitset = if is_positive_aura {
            &self.visible_positive_aura_slots_occupation
        } else {
            &self.visible_negative_aura_slots_occupation
        };

        (0..bitset.len()).find(|&idx| !bitset.contains(idx))
    }

    fn lock_slot(&mut self, slot: usize, is_positive_aura: bool) {
        let bitset = if is_positive_aura {
            &mut self.visible_positive_aura_slots_occupation
        } else {
            &mut self.visible_negative_aura_slots_occupation
        };

        bitset.set(slot, true);
    }
}

struct AuraApplication {
    aura: Aura,
    slot: Option<usize>,
}

impl AuraApplication {
    pub fn new(aura: Aura, slot: Option<usize>) -> Self {
        Self { aura, slot }
    }

    pub fn spell_id(&self) -> u32 {
        self.aura.spell_id
    }
}
