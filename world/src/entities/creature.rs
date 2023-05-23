use super::{entity::Entity, object_guid::ObjectGuid, position::WorldPosition};

pub struct Creature {
    guid: ObjectGuid,
    name: String,
    position: Option<WorldPosition>,
}

impl Creature {
    pub fn load(guid: &ObjectGuid) -> Self {
        Creature {
            guid: guid.clone(),
            name: "test npc".to_owned(),
            position: None,
        }
    }

    pub fn guid(&self) -> &ObjectGuid {
        &self.guid
    }

    pub fn position(&self) -> &WorldPosition {
        self.position
            .as_ref()
            .expect("Creature position uninitialized. Is the creature in world?")
    }
}

impl Entity for Creature {
    fn guid(&self) -> &ObjectGuid {
        self.guid()
    }

    fn name(&self) -> String {
        self.name.to_owned()
    }
}
