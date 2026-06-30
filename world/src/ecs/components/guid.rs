use std::sync::Arc;

use parking_lot::RwLock;
use shipyard::Component;

use crate::entities::{
    internal_values::InternalValues, object_guid::ObjectGuid, update_fields::ObjectFields,
};

#[derive(Component)]
pub struct Guid(pub ObjectGuid);

impl Guid {
    pub fn new(guid: ObjectGuid, internal_values: Arc<RwLock<InternalValues>>) -> Self {
        internal_values
            .write()
            .set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());
        Self(guid)
    }
}

impl std::ops::Deref for Guid {
    type Target = ObjectGuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Guid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
