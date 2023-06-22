use std::sync::Arc;

use enumflags2::{make_bitflags, BitFlags};
use parking_lot::RwLock;
use rand::{seq::SliceRandom, Rng};
use shipyard::Component;

use crate::{
    protocol::packets::SmsgCreateObject,
    repositories::creature::CreatureSpawnDbRecord,
    shared::constants::{HighGuidType, NpcFlags, ObjectTypeId, ObjectTypeMask},
    DataStore,
};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    update::{CreateData, MovementUpdateData, UpdateBlockBuilder, UpdateFlag, UpdateType},
    update_fields::{ObjectFields, UnitFields, UNIT_END},
};

#[derive(Component)]
pub struct Creature {
    guid: ObjectGuid,
    pub entry: u32,
    pub name: String,
    pub npc_flags: BitFlags<NpcFlags>,
    pub internal_values: Arc<RwLock<InternalValues>>,
}

impl Creature {
    pub fn from_spawn(
        creature_spawn: &CreatureSpawnDbRecord,
        data_store: Arc<DataStore>,
    ) -> Option<Self> {
        data_store
            .get_creature_template(creature_spawn.entry)
            .map(|template| {
                let mut rng = rand::thread_rng();

                let guid = ObjectGuid::with_entry(
                    HighGuidType::Unit,
                    creature_spawn.entry,
                    creature_spawn.guid,
                );
                let mut values = InternalValues::new(UNIT_END as usize);
                values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());

                let object_type = make_bitflags!(ObjectTypeMask::{Object | Unit}).bits();
                values.set_u32(ObjectFields::ObjectFieldType.into(), object_type);

                values.set_u32(ObjectFields::ObjectFieldEntry.into(), template.entry);

                values.set_f32(ObjectFields::ObjectFieldScaleX.into(), template.scale);

                values.set_u32(
                    UnitFields::UnitFieldLevel.into(),
                    rng.gen_range(template.min_level..=template.max_level),
                );

                let existing_model_ids: Vec<&u32> =
                    template.model_ids.iter().filter(|&&id| id != 0).collect();
                let display_id = existing_model_ids.choose(&mut rng).expect("rng error");
                values.set_u32(UnitFields::UnitFieldDisplayid.into(), **display_id);
                values.set_u32(UnitFields::UnitFieldNativedisplayid.into(), **display_id);
                // TODO: CombatReach must come from a DBC
                values.set_f32(UnitFields::UnitFieldCombatReach.into(), 1.5);

                values.set_u32(
                    UnitFields::UnitFieldFactiontemplate.into(),
                    template.faction_template_id,
                );

                values.set_u32(UnitFields::UnitNpcFlags.into(), template.npc_flags);
                values.set_u32(UnitFields::UnitFieldFlags.into(), template.unit_flags);
                values.set_u32(UnitFields::UnitDynamicFlags.into(), template.dynamic_flags);

                Creature {
                    guid,
                    entry: template.entry,
                    name: template.name.to_owned(),
                    npc_flags: unsafe { BitFlags::from_bits_unchecked(template.npc_flags) },
                    internal_values: Arc::new(RwLock::new(values)),
                }
            })
    }

    pub fn build_create_object(&self, movement: Option<MovementUpdateData>) -> SmsgCreateObject {
        let flags = make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition});
        let mut update_builder = UpdateBlockBuilder::new();

        let internal_values = self.internal_values.read();
        for index in 0..UNIT_END {
            let value = internal_values.get_u32(index as usize);
            if value != 0 {
                update_builder.add(index as usize, value);
            }
        }
        drop(internal_values);

        let blocks = update_builder.build();

        let update_data = vec![CreateData {
            update_type: UpdateType::CreateObject2,
            packed_guid: self.guid.as_packed(),
            object_type: ObjectTypeId::Unit,
            flags,
            movement,
            low_guid_part: None,
            high_guid_part: Some(HighGuidType::Unit as u32),
            blocks,
        }];

        SmsgCreateObject {
            updates_count: update_data.len() as u32,
            has_transport: false,
            updates: update_data,
        }
    }
}
