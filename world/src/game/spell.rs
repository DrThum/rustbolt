use shipyard::EntityId;

pub struct Spell {
    id: u32,
    target: EntityId,
}

impl<'a> Spell {
    pub fn new(id: u32, target: EntityId) -> Self {
        Self { id, target }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn target(&self) -> EntityId {
        self.target
    }
}
