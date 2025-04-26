use std::{sync::Arc, time::Instant};

use enumflags2::{make_bitflags, BitFlags};
use fixedbitset::FixedBitSet;
use log::warn;
use parking_lot::RwLock;
use shipyard::Component;

use crate::{
    datastore::data_types::SpellRecord,
    entities::{
        internal_values::InternalValues, object_guid::ObjectGuid, update_fields::UnitFields,
    },
    game::{
        aura::Aura, aura_effect_handler::AuraEffectHandlerArgs, spell::Spell,
        world_context::WorldContext,
    },
    protocol::{
        packets::{
            SmsgClearExtraAuraInfo, SmsgSetExtraAuraInfo, SmsgSetExtraAuraInfoNeedUpdate,
            SmsgUpdateAuraDuration,
        },
        server::ServerMessage,
    },
    session::world_session::WorldSession,
    shared::constants::{AuraEffect, AuraFlag, MAX_SPELL_EFFECTS, UNIT_AURAS_LIMIT},
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
        effect_index: usize,
        spell_record: Arc<SpellRecord>,
        caster_session: Option<Arc<WorldSession>>,
        target_session: Option<Arc<WorldSession>>,
        data_store: Arc<DataStore>,
    ) {
        match self
            .auras
            .iter_mut()
            .find(|aura| aura.spell_id() == spell.id() && aura.caster_guid() == spell.caster_guid())
        {
            Some(existing_aura) if !existing_aura.has_effect_index(effect_index) => {
                // Use the same slot for auras from the same spell and same caster
                existing_aura.aura.add_effect_index(effect_index);
            }
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

                let duration = spell_record
                    .base_duration(data_store.clone())
                    .unwrap_or_default();

                let aura = Aura::new(
                    spell.id(),
                    effect_index,
                    spell.caster(),
                    spell.caster_guid(),
                    target_entity_id,
                    target_guid,
                    duration,
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

                if let Some(slot) = slot {
                    if let Some(session) = target_session {
                        let packet = ServerMessage::new(SmsgUpdateAuraDuration {
                            slot: slot as u8,
                            duration_ms: duration.as_millis() as u32,
                        });

                        session.send(&packet).unwrap();

                        let packet = ServerMessage::new(SmsgSetExtraAuraInfo {
                            target_guid: target_guid.as_packed(),
                            slot: slot as u8,
                            spell_id: aura.spell_id,
                            max_duration_ms: duration.as_millis() as u32,
                            duration_ms: duration.as_millis() as u32,
                        });

                        session.send(&packet).unwrap();
                    }

                    if let Some(caster_session) = caster_session {
                        if spell.caster() != target_entity_id {
                            let packet = ServerMessage::new(SmsgSetExtraAuraInfoNeedUpdate {
                                target_guid: target_guid.as_packed(),
                                slot: slot as u8,
                                spell_id: aura.spell_id,
                                max_duration_ms: duration.as_millis() as u32,
                                duration_ms: duration.as_millis() as u32,
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

    // TODO: move this to the update_auras system
    pub fn update(
        &mut self,
        session: Option<Arc<WorldSession>>,
        world_context: Arc<WorldContext>,
        all_storages: &shipyard::AllStoragesViewMut,
    ) {
        let now = Instant::now();

        for index in (0..self.auras.len()).rev() {
            let aura_app = unsafe { self.auras.get_unchecked_mut(index) };

            if aura_app.state == AuraApplicationState::New {
                // Get the spell record
                let spell_record = world_context
                    .data_store
                    .get_spell_record(aura_app.spell_id())
                    .unwrap();

                // Apply each effect
                for effect_index in 0..MAX_SPELL_EFFECTS {
                    if !aura_app.has_effect_index(effect_index) {
                        continue;
                    }

                    if let Some(effect) =
                        AuraEffect::n(spell_record.effect_apply_aura_name[effect_index])
                    {
                        let handler = world_context.aura_effect_handler.get_handler(&effect);
                        handler(AuraEffectHandlerArgs {
                            world_context: world_context.clone(),
                            all_storages,
                        });
                    }
                }

                aura_app.state = AuraApplicationState::Active;
            }

            if aura_app.aura.is_expired(now) {
                if let Some(slot) = aura_app.slot {
                    // Update internal values for this specific slot
                    let mut values = self.internal_values.write();

                    values.set_u32(UnitFields::UnitFieldAura as usize + slot, 0);

                    let update_field_slot = slot / 4;
                    let update_field_offset = slot % 4;

                    values.set_u8(
                        UnitFields::UnitFieldAuraFlags as usize + update_field_slot,
                        update_field_offset,
                        0,
                    );

                    values.set_u8(
                        UnitFields::UnitFieldAuraLevels as usize + update_field_slot,
                        update_field_offset,
                        0,
                    );

                    values.set_u8(
                        UnitFields::UnitFieldAuraApplications as usize + update_field_slot,
                        update_field_offset,
                        0,
                    );

                    drop(values);

                    if let Some(ref session) = session {
                        let packet = ServerMessage::new(SmsgClearExtraAuraInfo {
                            target_guid: aura_app.aura.target_guid.as_packed(),
                            spell_id: aura_app.aura.spell_id,
                        });

                        session.send(&packet).unwrap();
                    }
                }

                self.auras.remove(index);
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
    state: AuraApplicationState,
}

impl AuraApplication {
    pub fn new(aura: Aura, slot: Option<usize>) -> Self {
        Self {
            aura,
            slot,
            state: AuraApplicationState::New,
        }
    }

    pub fn spell_id(&self) -> u32 {
        self.aura.spell_id
    }

    pub fn has_effect_index(&self, effect_index: usize) -> bool {
        self.aura.effect_mask().contains(effect_index)
    }

    pub fn caster_guid(&self) -> ObjectGuid {
        self.aura.caster_guid
    }
}

#[derive(Debug, PartialEq)]
enum AuraApplicationState {
    New,
    Active,
    Removing,
}
