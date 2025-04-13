use binrw::{binread, binwrite};
use opcode_derive::server_opcode;

use crate::entities::object_guid::ObjectGuid;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;

#[binread]
pub struct CmsgListInventory {
    pub vendor_guid: ObjectGuid,
}

#[binwrite]
#[server_opcode]
pub struct SmsgListInventory {
    pub vendor_guid: ObjectGuid,
    pub inventory_size: u8,
    pub items: Option<Vec<InventoryItem>>, // Some if inventory_size > 0
    pub error_code: Option<u32>,           // Some if inventory_size == 0
}

impl SmsgListInventory {
    pub fn empty(vendor_guid: ObjectGuid) -> Self {
        Self {
            vendor_guid,
            inventory_size: 0,
            items: None,
            error_code: Some(0),
        }
    }

    pub fn from_items(vendor_guid: ObjectGuid, items: Vec<InventoryItem>) -> Self {
        Self {
            vendor_guid,
            inventory_size: items.len() as u8,
            items: Some(items),
            error_code: None,
        }
    }
}

#[binwrite]
pub struct InventoryItem {
    pub index: u32, // 1-based
    pub item_id: u32,
    pub item_display_id: u32,
    pub item_count_at_vendor: u32, // 0xFFFFFFFF if <= 0
    pub price: u32,
    pub max_durability: u32,
    pub buy_count: u32,
    pub extended_cost_id: u32,
}
