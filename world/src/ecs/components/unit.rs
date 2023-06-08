use shipyard::{Component, EntityId};

#[derive(Component)]
pub struct Unit {
    target: Option<EntityId>,
}

impl Unit {
    pub fn new() -> Self {
        Self { target: None }
    }

    pub fn target(&self) -> Option<EntityId> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<EntityId>) {
        self.target = target;
    }
}
