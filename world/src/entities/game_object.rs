use shipyard::Component;

use super::object_guid::ObjectGuid;

#[derive(Component)]
pub struct GameObject {
    guid: ObjectGuid,
}
