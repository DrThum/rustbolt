use super::object_guid::ObjectGuid;

pub trait Entity {
    fn guid(&self) -> &ObjectGuid;
    fn name(&self) -> String;
}
