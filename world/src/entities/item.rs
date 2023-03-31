use enumflags2::make_bitflags;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::{repositories::item::ItemRepository, shared::constants::HighGuidType};

use super::{
    object_guid::ObjectGuid,
    update::{UpdatableEntity, UpdateData, UpdateFlag, UpdateType},
    ObjectTypeId,
};

pub struct Item {
    guid: ObjectGuid,
    entry: u32,
    owner_guid: Option<u64>,
}

impl Item {
    pub fn load(conn: &PooledConnection<SqliteConnectionManager>, guid: u64) -> Item {
        let item_db_record = ItemRepository::load(&conn, guid).expect("Item not found in DB");

        Item {
            guid: ObjectGuid::new(HighGuidType::ItemOrContainer, guid as u32),
            entry: item_db_record.entry,
            owner_guid: item_db_record.owned_guid,
        }
    }
}

impl UpdatableEntity for Item {
    fn get_create_data(&self) -> Vec<UpdateData> {
        let update_data = UpdateData {
            has_transport: false,
            update_type: UpdateType::CreateObject,
            packed_guid: self.guid.pack(),
            object_type: ObjectTypeId::Item,
            flags: make_bitflags!(UpdateFlag::{LowGuid | HighGuid}),
            movement: None,
            low_guid_part: Some(self.guid.counter()),
            high_guid_part: Some(HighGuidType::ItemOrContainer as u32),
            blocks: todo!(),
        };

        vec![update_data]
    }

    fn get_update_data(&self) -> Vec<UpdateData> {
        todo!()
    }
}
