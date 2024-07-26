use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use log::error;
use shipyard::{Component, EntityId};

use crate::game::spell::Spell;

#[derive(Component)]
pub struct SpellCast {
    current_ranged: Option<Arc<Spell>>,
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

    pub fn current_ranged(&self) -> Option<(Arc<Spell>, Instant)> {
        match (self.current_ranged.as_ref(), self.ranged_cast_end) {
            (None, None) => None,
            (Some(curr), Some(end)) => Some((curr.clone(), end)),
            _ => {
                error!("inconsistent state: current_ranged.is_some() != ranged_cast_end.is_some()");
                None
            }
        }
    }

    pub fn set_current_ranged(
        &mut self,
        spell_id: u32,
        duration: Duration,
        caster: EntityId,
        unit_target: Option<EntityId>,
        game_object_target: Option<EntityId>,
        power_cost: u32,
    ) {
        self.current_ranged = Some(Arc::new(Spell::new(
            spell_id,
            caster,
            unit_target,
            game_object_target,
            power_cost,
        )));
        self.ranged_cast_end = Some(Instant::now() + duration)
    }

    pub fn clean(&mut self) {
        self.current_ranged = None;
        self.ranged_cast_end = None;
    }
}
