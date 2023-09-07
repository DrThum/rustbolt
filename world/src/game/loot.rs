use rand::Rng;

#[derive(Copy, Clone)]
pub struct Loot {
    money: u32,
}

impl Loot {
    pub fn new() -> Self {
        Self { money: 0 }
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

    pub fn validate(self) -> Option<Self> {
        let has_money = self.money > 0;
        // let has_items
        let is_valid = has_money /* || has_items */;

        if is_valid {
            Some(self)
        } else {
            None
        }
    }
}
