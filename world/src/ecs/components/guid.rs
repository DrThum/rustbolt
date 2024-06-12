use std::sync::Arc;

use shipyard::Component;

use crate::entities::{
    internal_values::InternalValues, object_guid::ObjectGuid, update_fields::ObjectFields,
};

#[derive(Component)]
pub struct Guid(pub ObjectGuid);

impl Guid {
    pub fn new(guid: ObjectGuid, internal_values: Arc<InternalValues>) -> Self {
        internal_values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());
        Self { 0: guid }
    }
}
