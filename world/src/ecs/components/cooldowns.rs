use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use shipyard::Component;

#[derive(Component)]
pub struct Cooldowns {
    spell_cooldowns: HashMap<u32, SpellCooldown>,
}

impl Cooldowns {
    pub fn new(existing_cooldowns: HashMap<u32, SpellCooldown>) -> Self {
        Self {
            spell_cooldowns: existing_cooldowns,
        }
    }

    pub fn add_spell_cooldown(&mut self, spell_id: u32, cooldown: Duration, item_id: Option<u32>) {
        self.spell_cooldowns.insert(
            spell_id,
            SpellCooldown {
                end: SystemTime::now() + cooldown,
                item_id,
                synced_with_client: false,
            },
        );
    }

    pub fn list_mut(&mut self) -> impl Iterator<Item = (&u32, &mut SpellCooldown)> {
        self.spell_cooldowns.iter_mut()
    }

    pub fn list(&self) -> impl Iterator<Item = (&u32, &SpellCooldown)> {
        self.spell_cooldowns.iter()
    }
}

pub struct SpellCooldown {
    pub end: SystemTime,
    pub item_id: Option<u32>,
    pub synced_with_client: bool,
}
