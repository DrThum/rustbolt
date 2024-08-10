use core::fmt;
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum WowheadEntityType {
    Npc,
    Object,
    Item,
}

impl fmt::Display for WowheadEntityType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let display = match self {
            WowheadEntityType::Npc => "npc",
            WowheadEntityType::Object => "object",
            WowheadEntityType::Item => "item",
        };
        write!(f, "{display}")
    }
}

pub struct WowheadLootTable {
    pub entity_type: WowheadEntityType,
    pub id: u32,
    pub items: Vec<WowheadLootItem>,
}

#[derive(Debug)]
pub struct WowheadLootItem {
    pub id: u32,
    pub icon_url: String,
    pub name: String,
    pub loot_percent_chance: f32,
    pub min_count: Option<u32>,
    pub max_count: Option<u32>,
}
