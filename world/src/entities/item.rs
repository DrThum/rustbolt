use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use enumflags2::make_bitflags;

use crate::{
    game::world_context::WorldContext,
    shared::constants::{HighGuidType, ObjectTypeId, ObjectTypeMask},
};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    update::{
        CreateData, UpdateBlock, UpdateBlockBuilder, UpdateData, UpdateFlag, UpdateType,
        WorldEntity,
    },
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

    fn gen_create_data(&self) -> UpdateBlock {
        let mut update_builder = UpdateBlockBuilder::new();

        for index in 0..ITEM_END {
            let value = self.values.get_u32(index as usize);
            if value != 0 {
                update_builder.add(index as usize, value);
            }
        }

        update_builder.build()
    }
}

#[async_trait]
impl WorldEntity for Item {
    fn guid(&self) -> &ObjectGuid {
        self.guid()
    }

    fn name(&self) -> String {
        "TODO".to_owned()
    }

    async fn tick(&mut self, _diff: Duration, _world_context: Arc<WorldContext>) {}

    fn get_create_data(
        &self,
        _recipient_guid: u64,
        _world_context: Arc<WorldContext>,
    ) -> Vec<CreateData> {
        let update_data = CreateData {
            update_type: UpdateType::CreateObject,
            packed_guid: self.guid.as_packed(),
            object_type: ObjectTypeId::Item,
            flags: make_bitflags!(UpdateFlag::{LowGuid | HighGuid}),
            movement: None,
            low_guid_part: Some(self.guid.counter()),
            high_guid_part: Some(HighGuidType::ItemOrContainer as u32),
            blocks: self.gen_create_data(),
        };

        vec![update_data]
    }

    fn get_update_data(
        &self,
        _recipient_guid: u64,
        _world_context: Arc<WorldContext>,
    ) -> Vec<UpdateData> {
        todo!()
    }

    fn has_updates(&self) -> bool {
        self.values.has_dirty()
    }

    fn mark_up_to_date(&mut self) {
        self.values.reset_dirty();
    }

    fn modify_health(&mut self, _damage: i32) {
        panic!("attempt to call modify_health on an Item");
    }
}
