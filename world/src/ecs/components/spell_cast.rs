use std::time::{Duration, Instant};

use log::error;
use shipyard::Component;

#[derive(Component)]
pub struct SpellCast {
    current_ranged: Option<u32>,
    ranged_cast_end: Option<Instant>,
    // current melee: Option<u32>,
}

impl SpellCast {
    pub fn new() -> Self {
        Self {
            current_ranged: None,
            ranged_cast_end: None,
        }
    }

    pub fn current_ranged(&self) -> Option<(u32, Instant)> {
        match (self.current_ranged, self.ranged_cast_end) {
            (None, None) => None,
            (Some(curr), Some(end)) => Some((curr, end)),
            _ => {
                error!("inconsistent state: current_ranged.is_some() != ranged_cast_end.is_some()");
                None
            }
        }
    }

    pub fn set_current_ranged(&mut self, spell_id: u32, duration: Duration) {
        self.current_ranged = Some(spell_id);
        self.ranged_cast_end = Some(Instant::now() + duration)
    }

    pub fn clean(&mut self) {
        self.current_ranged = None;
        self.ranged_cast_end = None;
    }
}
