use shipyard::EntityId;

pub struct Spell {
    id: u32,
    caster: EntityId,
    unit_target: Option<EntityId>,
    game_object_target: Option<EntityId>,
    // item_target: EntityId, // TODO: We'll have to make Item a Component for this to work
    power_cost: u32,
}

impl Spell {
    pub fn new(
        id: u32,
        caster: EntityId,
        unit_target: Option<EntityId>,
        game_object_target: Option<EntityId>,
        power_cost: u32,
    ) -> Self {
        Self {
            id,
            caster,
            unit_target,
            game_object_target,
            power_cost,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn unit_target(&self) -> Option<EntityId> {
        self.unit_target
    }

    pub fn game_object_target(&self) -> Option<EntityId> {
        self.game_object_target
    }

    pub fn caster(&self) -> EntityId {
        self.caster
    }

    pub fn power_cost(&self) -> u32 {
        self.power_cost
    }
}
