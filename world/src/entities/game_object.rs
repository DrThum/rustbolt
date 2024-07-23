use std::sync::Arc;

use parking_lot::RwLock;
use shipyard::Component;

use crate::{
    game::world_context::WorldContext, repositories::game_object::GameObjectSpawnDbRecord,
    shared::constants::HighGuidType,
};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    update_fields::{ObjectFields, GAME_OBJECT_END},
};

#[derive(Component)]
pub struct GameObject {
    guid: ObjectGuid,
    pub internal_values: Arc<RwLock<InternalValues>>,
}

impl GameObject {
    pub fn from_spawn(
        game_object_spawn: &GameObjectSpawnDbRecord,
        world_context: Arc<WorldContext>,
    ) -> Option<Self> {
        let data_store = world_context.data_store.clone();
        data_store
            .get_game_object_template(game_object_spawn.entry)
            .map(|_template| {
                let guid = ObjectGuid::with_entry(
                    HighGuidType::Gameobject,
                    game_object_spawn.entry,
                    game_object_spawn.guid,
                );

                let mut values = InternalValues::new(GAME_OBJECT_END as usize);
                values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());

                GameObject {
                    guid,
                    internal_values: Arc::new(RwLock::new(values)),
                }
            })
    }
}
