use binrw::{binread, binwrite};
use opcode_derive::server_opcode;

use crate::entities::object_guid::ObjectGuid;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::BuyFailedReason;

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

#[binread]
pub struct CmsgBuyItem {
    pub vendor_guid: ObjectGuid,
    pub item_id: u32,
    pub count: u8,
    _unk: u8,
}

#[binwrite]
#[server_opcode]
pub struct SmsgBuyItem {
    pub vendor_guid: ObjectGuid,
    pub index: u32,     // 1-based
    pub new_count: u32, // 0xFFFFFFFF if item is in unlimited quantity
    pub bought_count: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgBuyFailed {
    pub vendor_guid: ObjectGuid,
    pub item_id: u32,
    pub param: Option<u32>,
    #[bw(map = |bfr: &BuyFailedReason| *bfr as u8)]
    pub fail_reason: BuyFailedReason,
}
