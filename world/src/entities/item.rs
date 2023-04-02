use enumflags2::make_bitflags;

use crate::shared::constants::{HighGuidType, ObjectTypeMask};

use super::{
    object_guid::ObjectGuid,
    update::{
        UpdatableEntity, UpdateBlock, UpdateBlockBuilder, UpdateData, UpdateFlag, UpdateType,
    },
    update_fields::{ItemFields, ObjectFields},
    ObjectTypeId,
};

pub struct Item {
    guid: ObjectGuid,
    entry: u32,
    owner_guid: Option<u64>,
}

impl Item {
    pub fn new(guid: u32, entry: u32, owner_guid: u64) -> Item {
        Item {
            guid: ObjectGuid::new(HighGuidType::ItemOrContainer, guid),
            entry,
            owner_guid: Some(owner_guid),
        }
    }

    pub fn guid(&self) -> &ObjectGuid {
        &self.guid
    }

    pub fn entry(&self) -> &u32 {
        &self.entry
    }

    fn gen_create_data(&self) -> UpdateBlock {
        let mut update_builder = UpdateBlockBuilder::new();
        let object_type = make_bitflags!(ObjectTypeMask::{Object | Item}).bits();

        update_builder.add_u64(ObjectFields::ObjectFieldGuid.into(), self.guid.raw());
        update_builder.add_u32(ObjectFields::ObjectFieldType.into(), object_type);
        update_builder.add_u32(ObjectFields::ObjectFieldEntry.into(), self.entry);
        update_builder.add_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0);
        if let Some(owner_guid) = self.owner_guid {
            update_builder.add_u64(ItemFields::ItemFieldOwner.into(), owner_guid);
            update_builder.add_u64(ItemFields::ItemFieldContained.into(), owner_guid);
        }
        update_builder.add_u32(ItemFields::ItemFieldStackCount.into(), 1);

        update_builder.build()
    }
}

impl UpdatableEntity for Item {
    fn get_create_data(&self) -> Vec<UpdateData> {
        let update_data = UpdateData {
            update_type: UpdateType::CreateObject,
            packed_guid: self.guid.as_packed(),
            object_type: ObjectTypeId::Item,
            flags: make_bitflags!(UpdateFlag::{LowGuid | HighGuid}),
            movement: None,
            low_guid_part: Some(self.guid.counter()),
            high_guid_part: Some(HighGuidType::ItemOrContainer as u32),
            blocks: vec![self.gen_create_data()],
        };

        // vec![update_data]
        Vec::new()
    }

    fn get_update_data(&self) -> Vec<UpdateData> {
        todo!()
    }
}
