use rand::{prelude::Distribution, Rng};

use crate::datastore::data_types::{LootGroup, LootItem, LootTable};

#[derive(Clone)]
pub struct Loot {
    money: u32,
    items: Vec<ItemInLoot>,
}

#[derive(Clone)]
pub struct ItemInLoot {
    pub index: u32, // Index in the loot window when the loot is generated (doesn't change when items
    // are looted)
    pub item_id: u32,
    pub count: u32,
    pub random_suffix: u32,
    pub random_property_id: u32,
}

impl Loot {
    pub fn new() -> Self {
        Self {
            money: 0,
            items: vec![],
        }
    }

    pub fn add_money(&mut self, min: u32, max: u32) {
        let min = min.min(max);
        let max = max.max(min);

        if max > 0 {
            if min == max {
                self.money = max;
            } else if max - min < 32700 {
                self.money = rand::thread_rng().gen_range(min..=max);
            } else {
                let min = min / 256;
                let max = max / 256;
                self.money = rand::thread_rng().gen_range(min..=max) * 256;
            }
        }
    }

    pub fn money(&self) -> u32 {
        self.money
    }

    pub fn remove_money(&mut self) {
        self.money = 0;
    }

    pub fn add_item(
        &mut self,
        item_id: u32,
        count: u32, /*, random_suffix: u32, random_property_id: u32*/
    ) {
        self.items.push(ItemInLoot {
            index: self.items.len() as u32,
            item_id,
            count,
            random_suffix: 0,
            random_property_id: 0,
        })
    }

    pub fn items(&self) -> &Vec<ItemInLoot> {
        &self.items
    }

    pub fn remove_item(&mut self, index: u32) {
        self.items.retain(|item| item.index != index);
    }

    pub fn is_empty(&self) -> bool {
        self.money == 0
    }
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
            .map(|group| {
                let num_rolls = group.num_rolls.random_value();

                let mut items: Vec<LootItem> = Vec::new();
                for _ in 0..num_rolls {
                    items.push(group.generate_loot());
                }

                items
            })
            .flatten()
            .collect()
    }
}

impl LootGroup {
    pub fn generate_loot(&self) -> LootItem {
        let mut rng = rand::thread_rng();
        self.items[self.distribution.sample(&mut rng)]
    }
}
