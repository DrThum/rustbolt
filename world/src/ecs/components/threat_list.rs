use std::collections::HashMap;

use shipyard::Component;

use crate::entities::object_guid::ObjectGuid;

#[derive(Component)]
pub struct ThreatList {
    threat_list: HashMap<ObjectGuid, f32>,
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

    pub fn modify_threat(&mut self, guid: ObjectGuid, amount: f32) {
        self.threat_list
            .entry(guid)
            .and_modify(|threat| *threat += amount)
            .or_insert(amount);
    }

    pub fn remove_from_threat_list(&mut self, guid: &ObjectGuid) {
        self.threat_list.remove(guid);
    }
}
