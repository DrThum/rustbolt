use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;
use shipyard::{EntityId, Unique};

use crate::entities::object_guid::ObjectGuid;

pub struct EntityManager {
    ecs_entities: RwLock<HashMap<ObjectGuid, EntityId>>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            ecs_entities: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, guid: ObjectGuid, entity: EntityId) {
        self.ecs_entities.write().insert(guid, entity);
    }

    pub fn lookup(&self, guid: &ObjectGuid) -> Option<EntityId> {
        self.ecs_entities.read().get(guid).copied()
    }

    pub fn remove(&self, guid: &ObjectGuid) -> Option<EntityId> {
        self.ecs_entities.write().remove(guid)
    }
}

#[derive(Unique)]
pub struct WrappedEntityManager(pub Arc<EntityManager>);
