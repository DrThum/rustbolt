use shipyard::Component;

use crate::entities::object_guid::ObjectGuid;

#[derive(Component)]
pub struct Guid(pub ObjectGuid);

impl Guid {
    pub fn new(guid: ObjectGuid) -> Self {
        Self { 0: guid }
    }
}
