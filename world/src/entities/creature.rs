use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use enumflags2::make_bitflags;
use rand::{seq::SliceRandom, Rng};

use crate::{
    game::world_context::WorldContext,
    repositories::creature::CreatureSpawnDbRecord,
    shared::constants::{HighGuidType, ObjectTypeId, ObjectTypeMask},
    DataStore,
};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    position::{Position, WorldPosition},
    update::{
        CreateData, MovementUpdateData, UpdateBlock, UpdateBlockBuilder, UpdateData, UpdateFlag,
        UpdateType, WorldEntity,
    },
    update_fields::{ObjectFields, UnitFields, UNIT_END},
};

pub struct Creature {
    guid: ObjectGuid,
    name: String,
    values: InternalValues,
    position: Option<WorldPosition>,
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

                values.set_u32(UnitFields::UnitFieldHealth.into(), 100); // TODO
                values.set_u32(UnitFields::UnitFieldMaxhealth.into(), 100); // TODO

                values.set_u32(
                    UnitFields::UnitFieldFactiontemplate.into(),
                    template.faction_template_id,
                );

                values.set_u32(UnitFields::UnitNpcFlags.into(), template.npc_flags);
                values.set_u32(UnitFields::UnitFieldFlags.into(), template.unit_flags);
                values.set_u32(UnitFields::UnitDynamicFlags.into(), template.dynamic_flags);

                Creature {
                    guid: guid.clone(),
                    name: template.name.to_owned(),
                    values,
                    position: Some(WorldPosition {
                        map: creature_spawn.map,
                        zone: 1, // FIXME: calculate from position and terrain?
                        x: creature_spawn.position_x,
                        y: creature_spawn.position_y,
                        z: creature_spawn.position_z,
                        o: creature_spawn.orientation,
                    }),
                }
            })
    }

    pub fn has_changed_since_last_update(&self) -> bool {
        self.values.has_dirty()
    }

    pub fn mark_as_up_to_date(&mut self) {
        self.values.reset_dirty()
    }

    pub fn guid(&self) -> &ObjectGuid {
        &self.guid
    }

    pub fn position(&self) -> &WorldPosition {
        self.position
            .as_ref()
            .expect("Creature position uninitialized. Is the creature in world?")
    }

    fn gen_create_data(&self) -> UpdateBlock {
        let mut update_builder = UpdateBlockBuilder::new();

        for index in 0..UNIT_END {
            let value = self.values.get_u32(index as usize);
            if value != 0 {
                update_builder.add(index as usize, value);
            }
        }

        update_builder.build()
    }

    fn gen_update_data(&self) -> UpdateBlock {
        let mut update_builder = UpdateBlockBuilder::new();

        for index in self.values.get_dirty_indexes() {
            let value = self.values.get_u32(index as usize);
            update_builder.add(index as usize, value);
        }

        update_builder.build()
    }
}

#[async_trait]
impl WorldEntity for Creature {
    fn guid(&self) -> &ObjectGuid {
        self.guid()
    }

    fn name(&self) -> String {
        self.name.to_owned()
    }

    async fn tick(&mut self, _diff: Duration, _world_context: Arc<WorldContext>) {}

    fn get_create_data(
        &self,
        _recipient_guid: u64,
        world_context: std::sync::Arc<crate::game::world_context::WorldContext>,
    ) -> Vec<super::update::CreateData> {
        let movement = Some(MovementUpdateData {
            movement_flags: 0,  // 0x02000000, // TEMP: Flying
            movement_flags2: 0, // Always 0 in 2.4.3
            timestamp: world_context.game_time().as_millis() as u32, // Will overflow every 49.7 days
            position: Position {
                // FIXME: Into impl?
                x: self.position().x,
                y: self.position().y,
                z: self.position().z,
                o: self.position().o,
            },
            // pitch: Some(0.0),
            pitch: None,
            fall_time: 0,
            speed_walk: 2.5,
            speed_run: 7.0,
            speed_run_backward: 4.5,
            speed_swim: 4.722222,
            speed_swim_backward: 2.5,
            speed_flight: 70.0,
            speed_flight_backward: 4.5,
            speed_turn: 3.141594,
        });

        let flags = make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition});

        vec![CreateData {
            update_type: UpdateType::CreateObject2,
            packed_guid: self.guid().as_packed(),
            object_type: ObjectTypeId::Unit,
            flags,
            movement,
            low_guid_part: None,
            high_guid_part: Some(HighGuidType::Unit as u32),
            blocks: self.gen_create_data(),
        }]
    }

    fn has_updates(&self) -> bool {
        self.values.has_dirty()
    }

    fn mark_up_to_date(&mut self) {
        self.values.reset_dirty();
    }

    fn get_update_data(
        &self,
        _recipient_guid: u64,
        _world_context: std::sync::Arc<crate::game::world_context::WorldContext>,
    ) -> Vec<super::update::UpdateData> {
        vec![UpdateData {
            update_type: UpdateType::Values,
            packed_guid: self.guid().as_packed(),
            blocks: self.gen_update_data(),
        }]
    }
}
