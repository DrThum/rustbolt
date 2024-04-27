use binrw::{binread, binwrite};
use opcode_derive::server_opcode;

use crate::entities::object_guid::ObjectGuid;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::{LootSlotType, LootType};

#[binread]
pub struct CmsgLoot {
    pub target_guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgLootResponse {
    pub target_guid: u64,
    #[bw(map = |lt: &LootType| *lt as u8)]
    pub loot_type: LootType,
    pub money: u32,
    pub item_count: u8,
    pub items: Vec<LootResponseItem>,
}

impl SmsgLootResponse {
    pub fn build(
        target_guid: &ObjectGuid,
        loot_type: LootType,
        money: u32,
        items: Vec<LootResponseItem>,
    ) -> Self {
        Self {
            target_guid: target_guid.raw(),
            loot_type,
            money,
            item_count: items.len() as u8,
            items,
        }
    }
}

#[binwrite]
pub struct LootResponseItem {
    pub index: u8, // Index in the loot window
    pub id: u32,
    pub count: u32,
    pub display_info_id: u32, // From ItemDisplayInfo.dbc
    pub random_suffix: u32,
    pub random_property_id: u32,
    #[bw(map = |lst: &LootSlotType| *lst as u8)]
    pub slot_type: LootSlotType,
}

#[binwrite]
#[server_opcode]
pub struct SmsgLootMoneyNotify {
    pub money: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgLootClearMoney {}

#[binread]
pub struct CmsgLootRelease {
    pub looted_guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgLootReleaseResponse {
    pub looted_guid: u64,
    pub unk: u8, // Always 1
}

impl SmsgLootReleaseResponse {
    pub fn build(looted_guid: u64) -> Self {
        Self {
            looted_guid,
            unk: 1,
        }
    }
}

#[binread]
pub struct CmsgAutostoreLootItem {
    pub loot_index: u8,
}

#[binwrite]
#[server_opcode]
pub struct SmsgLootRemoved {
    pub loot_index: u8,
}
