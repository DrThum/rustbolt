use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::repositories::item::ItemRepository;

use super::{
    update::{UpdatableEntity, UpdateData, UpdateType},
    ObjectTypeId,
};

pub struct Item {
    guid: u64,
    entry: u32,
    owner_guid: Option<u64>,
}

impl Item {
    pub fn load(conn: &PooledConnection<SqliteConnectionManager>, guid: u64) -> Item {
        let item_db_record = ItemRepository::load(&conn, guid).expect("Item not found in DB");

        Item {
            guid,
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
            packed_guid: todo!(),
            object_type: ObjectTypeId::Item,
            flags: todo!(),
            movement_flags: 0,
            position: todo!(), // TODO: make this dependent on UPDATEFLAGS_HAS_POSITION
            fall_time: todo!(),
            speed_walk: todo!(),
            speed_run: todo!(),
            speed_run_backward: todo!(),
            speed_swim: todo!(),
            speed_swim_backward: todo!(),
            speed_flight: todo!(),
            speed_flight_backward: todo!(),
            speed_turn: todo!(),
            blocks: todo!(),
        };

        vec![update_data]
    }

    fn get_update_data(&self) -> Vec<UpdateData> {
        todo!()
    }
}
