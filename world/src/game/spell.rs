use shipyard::EntityId;

pub struct Spell {
    id: u32,
    caster: EntityId,
    target: EntityId,
}

impl<'a> Spell {
    pub fn new(id: u32, caster: EntityId, target: EntityId) -> Self {
        Self { id, caster, target }
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
}
