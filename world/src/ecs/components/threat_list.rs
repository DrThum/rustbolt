use std::collections::HashMap;

use shipyard::{Component, EntityId};

#[derive(Component)]
pub struct ThreatList {
    threat_list: HashMap<EntityId, f32>,
}

impl ThreatList {
    pub fn new() -> Self {
        Self {
            threat_list: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.threat_list.is_empty()
    }

    pub fn modify_threat(&mut self, entity_id: EntityId, amount: f32) {
        self.threat_list
            .entry(entity_id)
            .and_modify(|threat| *threat += amount)
            .or_insert(amount);
    }

    pub fn remove_from_threat_list(&mut self, entity_id: &EntityId) {
        self.threat_list.remove(entity_id);
    }

    pub fn threat_list(&self) -> HashMap<EntityId, f32> {
        self.threat_list.clone()
    }
}
