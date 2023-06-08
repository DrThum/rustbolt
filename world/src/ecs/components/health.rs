use shipyard::Component;

#[derive(Component)]
pub struct Health {
    current: u32,
    max: u32,
}

impl Health {
    pub fn new(current: u32, max: u32) -> Self {
        Self { current, max }
    }

    pub fn apply_damage(&mut self, damage: u32) {
        self.current = self.current.saturating_sub(damage).min(self.max);
    }

    // pub fn apply_healing(&mut self, healing: u32) {
    //     self.current = self.current.saturating_add(healing);
    // }

    // pub fn current(&self) -> u32 {
    //     self.current
    // }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }
}
