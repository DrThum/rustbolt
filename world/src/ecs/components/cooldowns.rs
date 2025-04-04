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
    pub fn new(existing_cooldowns: HashMap<u32, SystemTime>) -> Self {
        let mut spell_cooldowns = HashMap::new();
        for (spell_id, end) in existing_cooldowns {
            spell_cooldowns.insert(
                spell_id,
                SpellCooldown {
                    end,
                    synced_with_client: true, // Mark the cooldowns as synced with the client because they are sent in SMSG_INITIAL_SPELLS
                },
            );
        }

        Self { spell_cooldowns }
    }

    pub fn add_spell_cooldown(&mut self, spell_id: u32, cooldown: Duration) {
        self.spell_cooldowns.insert(
            spell_id,
            SpellCooldown {
                end: SystemTime::now() + cooldown,
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

// TODO: Add item_id: Option<u32>
pub struct SpellCooldown {
    pub end: SystemTime,
    pub synced_with_client: bool,
}
