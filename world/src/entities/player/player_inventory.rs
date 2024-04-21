use std::collections::HashMap;

use crate::entities::{item::Item, update::CreateData};

pub struct PlayerInventory {
    items: HashMap<u32, Item>, // Key is slot
}

impl PlayerInventory {
    pub fn new(items: HashMap<u32, Item>) -> Self {
        Self { items }
    }

    pub fn build_create_data(&self) -> Vec<CreateData> {
        self.items
            .iter()
            .map(|item| item.1.build_create_data())
            .collect()
    }

    pub fn get(&self, slot: u32) -> Option<&Item> {
        self.items.get(&slot)
    }
}
