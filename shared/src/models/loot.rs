use std::collections::HashSet;

use rand::{
    distributions::{Distribution, WeightedIndex},
    Rng,
};
use serde::{Deserialize, Serialize};

use crate::utils::value_range::ValueRange;

#[derive(Serialize)]
pub struct LootTable {
    pub id: u32,
    // pub description: Option<String>,
    pub groups: Vec<LootGroup>,
}

impl LootTable {
    pub fn generate_loots(&self) -> Vec<LootItem> {
        let mut rng = rand::thread_rng();

        self.groups
            .iter()
            .filter(|group| {
                let rolled_chance: f32 = rng.gen_range(0.0..100.0);

                group.chance >= rolled_chance
            })
            .flat_map(|group| {
                let num_rolls = group.num_rolls.random_value();

                let mut items: Vec<LootItem> = Vec::new();
                for _ in 0..num_rolls {
                    items.push(group.generate_loot());
                }

                items
            })
            .collect()
    }

    // NOTE: Maybe at some point we'll return a Vec<(ItemId, ConditionId)>
    pub fn get_all_possible_item_ids(&self) -> HashSet<u32> {
        self.groups
            .iter()
            .flat_map(|group| group.items.iter().map(|item| item.item_id))
            .collect()
    }
}

#[derive(Serialize)]
pub struct LootGroup {
    pub id: u32,
    pub chance: f32, // TODO: Make it a type?
    pub num_rolls: ValueRange<u8>,
    pub items: Vec<LootItem>,
    pub condition_id: Option<u32>,
    #[serde(skip_serializing)]
    pub distribution: WeightedIndex<f32>,
}

impl LootGroup {
    pub fn generate_loot(&self) -> LootItem {
        let mut rng = rand::thread_rng();
        self.items[self.distribution.sample(&mut rng)]
    }
}

#[derive(Copy, Clone, Serialize)]
pub struct LootItem {
    pub item_id: u32, // item_templates.entry
    pub chance: f32,  // TODO: Make it a type?
    pub count: ValueRange<u8>,
    pub condition_id: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateLootItem {
    pub item_id: u32,
    pub chance: f32,
    pub count: ValueRange<u32>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateLootGroup {
    pub id: Option<u32>,
    pub chance: f32,
    pub items: Vec<UpdateLootItem>,
    pub num_rolls: ValueRange<u32>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateLootTable {
    pub id: u32,
    pub groups: Vec<UpdateLootGroup>,
}
