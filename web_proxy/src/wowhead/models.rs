use core::fmt;

use serde::Serialize;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Serialize)]
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

impl TryFrom<String> for WowheadEntityType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "npc" | "creature" => Ok(Self::Npc),
            "gameobject" | "object" => Ok(Self::Object),
            "item" => Ok(Self::Item),
            value => Err(format!("unexpected WowheadEntityType {}", value)),
        }
    }
}

#[derive(Serialize)]
pub struct WowheadLootTable {
    pub entity_type: WowheadEntityType,
    pub id: u32,
    pub items: Vec<WowheadLootItem>,
}

#[derive(Debug, Serialize)]
pub struct WowheadLootItem {
    pub id: u32,
    pub icon_url: String,
    pub name: String,
    pub loot_percent_chance: f32,
    pub min_count: Option<u32>,
    pub max_count: Option<u32>,
}
