use std::time::Duration;

use shipyard::{EntityId, Unique};

#[derive(Unique, Default)]
pub struct DeltaTime(pub Duration);

impl std::ops::Deref for DeltaTime {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DeltaTime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct UnitDied {
    pub killer: EntityId,
    pub victim: EntityId,
}

#[derive(Unique, Default)]
pub struct CombatEvents(Vec<UnitDied>);

impl CombatEvents {
    pub fn push(&mut self, event: UnitDied) {
        self.0.push(event);
    }

    pub fn drain(&mut self) -> Vec<UnitDied> {
        std::mem::take(&mut self.0)
    }
}
