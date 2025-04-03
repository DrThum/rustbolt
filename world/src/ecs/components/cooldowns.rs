use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use shipyard::Component;

#[derive(Component)]
pub struct Cooldowns {
    spell_cooldowns: HashMap<u32, SpellCooldown>,
}

impl Cooldowns {
    pub fn new() -> Self {
        Self {
            spell_cooldowns: HashMap::new(),
        }
    }

    pub fn add_spell_cooldown(&mut self, spell_id: u32, cooldown: Duration) {
        self.spell_cooldowns.insert(
            spell_id,
            SpellCooldown {
                end: Instant::now() + cooldown,
                synced_with_client: false,
            },
        );
    }

    pub fn list_mut(&mut self) -> HashMap<&u32, &mut SpellCooldown> {
        self.spell_cooldowns.iter_mut().collect()
    }

    // TODO: handle spell categories (add a cooldown for all spells of a given category)
}

pub struct SpellCooldown {
    pub end: Instant,
    pub synced_with_client: bool,
}
