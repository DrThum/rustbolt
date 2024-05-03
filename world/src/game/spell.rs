use shipyard::EntityId;

pub struct Spell {
    id: u32,
    caster: EntityId,
    target: EntityId,
    power_cost: u32,
}

impl<'a> Spell {
    pub fn new(id: u32, caster: EntityId, target: EntityId, power_cost: u32) -> Self {
        Self {
            id,
            caster,
            target,
            power_cost,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn target(&self) -> EntityId {
        self.target
    }

    pub fn caster(&self) -> EntityId {
        self.caster
    }

    pub fn power_cost(&self) -> u32 {
        self.power_cost
    }
}
