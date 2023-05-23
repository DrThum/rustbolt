use enumflags2::make_bitflags;

use crate::shared::constants::{HighGuidType, ObjectTypeId, ObjectTypeMask};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    position::{Position, WorldPosition},
    update::{
        CreateData, MovementUpdateData, UpdatableEntity, UpdateBlock, UpdateBlockBuilder,
        UpdateData, UpdateFlag, UpdateType,
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
    pub fn load(guid: &ObjectGuid) -> Self {
        let mut values = InternalValues::new(UNIT_END as usize);
        values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());

        let object_type = make_bitflags!(ObjectTypeMask::{Object | Unit}).bits();
        values.set_u32(ObjectFields::ObjectFieldType.into(), object_type);

        values.set_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0);

        values.set_u32(UnitFields::UnitFieldLevel.into(), 1);

        values.set_u32(UnitFields::UnitFieldDisplayid.into(), 16633);
        values.set_u32(UnitFields::UnitFieldNativedisplayid.into(), 16633);

        values.set_u32(UnitFields::UnitFieldHealth.into(), 100);
        values.set_u32(UnitFields::UnitFieldMaxhealth.into(), 100);

        Creature {
            guid: guid.clone(),
            name: "test npc".to_owned(),
            values,
            position: Some(WorldPosition {
                map: 0,
                zone: 1,
                x: -6252.7919,
                y: 339.8356,
                z: 382.4590,
                o: 0.2260,
            }),
        }
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

impl UpdatableEntity for Creature {
    fn guid(&self) -> &ObjectGuid {
        self.guid()
    }

    fn name(&self) -> String {
        self.name.to_owned()
    }

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
