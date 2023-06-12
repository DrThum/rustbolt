use enumflags2::make_bitflags;

use crate::shared::constants::{HighGuidType, ObjectTypeId, ObjectTypeMask};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    update::{CreateData, UpdateBlockBuilder, UpdateFlag, UpdateType},
    update_fields::{ItemFields, ObjectFields, ITEM_END},
};

pub struct Item {
    guid: ObjectGuid,
    values: InternalValues,
}

impl Item {
    pub fn new(guid: u32, entry: u32, owner_guid: u64, stack_count: u32) -> Item {
        let guid = ObjectGuid::new(HighGuidType::ItemOrContainer, guid);
        let object_type = make_bitflags!(ObjectTypeMask::{Object | Item}).bits();

        let mut values = InternalValues::new(ITEM_END as usize);

        values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());
        values.set_u32(ObjectFields::ObjectFieldType.into(), object_type);
        values.set_u32(ObjectFields::ObjectFieldEntry.into(), entry);
        values.set_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0);
        values.set_u64(ItemFields::ItemFieldOwner.into(), owner_guid);
        values.set_u64(ItemFields::ItemFieldContained.into(), owner_guid); // Not in all cases
        values.set_u32(ItemFields::ItemFieldStackCount.into(), stack_count);

        Item { guid, values }
    }

    pub fn guid(&self) -> &ObjectGuid {
        &self.guid
    }

    pub fn entry(&self) -> u32 {
        self.values.get_u32(ObjectFields::ObjectFieldEntry.into())
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
            low_guid_part: Some(self.guid.counter()),
            high_guid_part: Some(HighGuidType::ItemOrContainer as u32),
            blocks,
        }
    }
}
