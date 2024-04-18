use rand::Rng;

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
