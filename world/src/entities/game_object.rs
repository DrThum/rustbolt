use std::sync::Arc;

use enumflags2::make_bitflags;
use parking_lot::RwLock;
use shipyard::Component;

use crate::{
    game::{map_manager::MapKey, world_context::WorldContext},
    protocol::packets::SmsgCreateObject,
    repositories::game_object::GameObjectSpawnDbRecord,
    shared::constants::{HighGuidType, ObjectTypeId, ObjectTypeMask},
};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    position::WorldPosition,
    update::{CreateData, PositionUpdateData, UpdateBlockBuilder, UpdateFlag, UpdateType},
    update_fields::{ObjectFields, GAME_OBJECT_END},
};

#[derive(Component)]
pub struct GameObject {
    guid: ObjectGuid,
    pub internal_values: Arc<RwLock<InternalValues>>,
    pub spawn_position: WorldPosition,
}

impl GameObject {
    pub fn from_spawn(
        game_object_spawn: &GameObjectSpawnDbRecord,
        world_context: Arc<WorldContext>,
    ) -> Option<Self> {
        let data_store = world_context.data_store.clone();
        data_store
            .get_game_object_template(game_object_spawn.entry)
            .map(|template| {
                let guid = ObjectGuid::with_entry(
                    HighGuidType::Gameobject,
                    game_object_spawn.entry,
                    game_object_spawn.guid,
                );
                let spawn_position = WorldPosition {
                    map_key: MapKey::for_continent(game_object_spawn.map), // TODO: MapKey for dungeon
                    zone: 0, // TODO: Calculate zone from terrain files
                    x: game_object_spawn.position_x,
                    y: game_object_spawn.position_y,
                    z: game_object_spawn.position_z,
                    o: game_object_spawn.orientation,
                };

                let mut values = InternalValues::new(GAME_OBJECT_END as usize);
                values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());

                let object_type = make_bitflags!(ObjectTypeMask::{Object | Gameobject}).bits();
                values.set_u32(ObjectFields::ObjectFieldType.into(), object_type);

                values.set_u32(ObjectFields::ObjectFieldEntry.into(), template.entry);

                GameObject {
                    guid,
                    internal_values: Arc::new(RwLock::new(values)),
                    spawn_position,
                }
            })
    }

    pub fn build_create_object(&self) -> SmsgCreateObject {
        let flags = make_bitflags!(UpdateFlag::{HighGuid | LowGuid | HasPosition});
        let mut update_builder = UpdateBlockBuilder::new();

        let internal_values = self.internal_values.read();
        for index in 0..GAME_OBJECT_END {
            let value = internal_values.get_u32(index as usize);
            if value != 0 {
                update_builder.add(index as usize, value);
            }
        }
        drop(internal_values);

        let blocks = update_builder.build();

        // FIXME: it's different for transports
        let position = Some(PositionUpdateData {
            position_x: self.spawn_position.x,
            position_y: self.spawn_position.y,
            position_z: self.spawn_position.z,
            orientation: self.spawn_position.o,
        });

        let update_data = vec![CreateData {
            update_type: UpdateType::CreateObject,
            packed_guid: self.guid.as_packed(),
            object_type: ObjectTypeId::GameObject,
            flags,
            movement: None,
            position,
            low_guid_part: Some(self.guid.counter()),
            high_guid_part: Some(self.guid.high_part() as u32),
            blocks,
        }];

        SmsgCreateObject {
            updates_count: update_data.len() as u32,
            has_transport: false,
            updates: update_data,
        }
    }
}
