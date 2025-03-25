use enumflags2::make_bitflags;
use log::warn;

use crate::shared::constants::{HighGuidType, ObjectTypeId, ObjectTypeMask};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    update::{CreateData, UpdateBlockBuilder, UpdateData, UpdateFlag, UpdateType},
    update_fields::{ItemFields, ObjectFields, ITEM_END},
};

pub struct Item {
    guid: ObjectGuid,
    values: InternalValues,
    needs_db_save: bool,
}

impl Item {
    pub fn new(
        guid: u32,
        entry: u32,
        owner_guid: u64,
        stack_count: u32,
        loaded_from_db: bool,
    ) -> Item {
        let guid = ObjectGuid::new(HighGuidType::ItemOrContainer, guid);
        let object_type = make_bitflags!(ObjectTypeMask::{Object | Item}).bits();

        let mut values = InternalValues::new(ITEM_END as usize);

        values
            .set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw())
            .set_u32(ObjectFields::ObjectFieldType.into(), object_type)
            .set_u32(ObjectFields::ObjectFieldEntry.into(), entry)
            .set_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0)
            .set_u64(ItemFields::ItemFieldOwner.into(), owner_guid)
            // TODO: Not in all cases
            .set_u64(ItemFields::ItemFieldContained.into(), owner_guid)
            .set_u32(ItemFields::ItemFieldStackCount.into(), stack_count);

        Item {
            guid,
            values,
            needs_db_save: !loaded_from_db,
        }
    }

    pub fn guid(&self) -> &ObjectGuid {
        &self.guid
    }

    pub fn entry(&self) -> u32 {
        self.values.get_u32(ObjectFields::ObjectFieldEntry.into())
    }

    pub fn stack_count(&self) -> u32 {
        self.values.get_u32(ItemFields::ItemFieldStackCount.into())
    }

    pub fn change_stack_count(&mut self, diff: i32) {
        let new_stack_count = self
            .stack_count()
            .checked_add_signed(diff)
            .unwrap_or_else(|| {
                warn!(
                "[BUG] attempt to set item stack count to a negative amount, setting to 0 instead"
            );
                0
            });
        self.values
            .set_u32(ItemFields::ItemFieldStackCount.into(), new_stack_count);

        self.needs_db_save = true;
    }

    pub fn needs_db_save(&self) -> bool {
        self.needs_db_save
    }

    pub fn mark_saved(&mut self) {
        self.needs_db_save = false;
    }

    pub fn build_create_data(&self) -> CreateData {
        let mut update_builder = UpdateBlockBuilder::new();

        for index in 0..ITEM_END {
            let value = self.values.get_u32(index as usize);
            if value != 0 {
                update_builder.add(index as usize, value);
            }
        }

        let blocks = update_builder.build();

        CreateData {
            update_type: UpdateType::CreateObject,
            packed_guid: self.guid.as_packed(),
            object_type: ObjectTypeId::Item,
            flags: make_bitflags!(UpdateFlag::{LowGuid | HighGuid}),
            movement: None,
            position: None,
            low_guid_part: Some(self.guid.counter()),
            high_guid_part: Some(HighGuidType::ItemOrContainer as u32),
            blocks,
        }
    }

    pub fn build_update_data_and_reset(&mut self) -> Option<UpdateData> {
        if !self.values.has_dirty() {
            return None;
        }

        let mut update_builder = UpdateBlockBuilder::new();

        for index in self.values.get_dirty_indexes() {
            let value = self.values.get_u32(index);
            update_builder.add(index, value);
        }

        let blocks = update_builder.build();

        self.values.reset_dirty();

        Some(UpdateData {
            update_type: UpdateType::Values,
            packed_guid: self.guid.as_packed(),
            blocks,
        })
    }
}
