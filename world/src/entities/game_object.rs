use std::sync::Arc;

use enumflags2::make_bitflags;
use parking_lot::RwLock;
use shipyard::Component;

use crate::{
    game::{map_manager::MapKey, world_context::WorldContext},
    protocol::packets::{SmsgCreateObject, SmsgUpdateObject},
    repositories::game_object::GameObjectSpawnDbRecord,
    shared::constants::{
        GameObjectDynamicLowFlags, HighGuidType, ObjectTypeId, ObjectTypeMask, PlayerQuestStatus,
    },
    DataStore,
};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    player::Player,
    position::WorldPosition,
    update::{
        CreateData, PositionUpdateData, UpdateBlockBuilder, UpdateData, UpdateFlag, UpdateType,
    },
    update_fields::{GameObjectFields, ObjectFields, GAME_OBJECT_END},
};

#[derive(Component)]
pub struct GameObject {
    guid: ObjectGuid,
    relevant_quest_ids: Vec<(u32, PlayerQuestStatus)>,
    data_store: Arc<DataStore>,
    pub internal_values: Arc<RwLock<InternalValues>>,
    pub spawn_position: WorldPosition,
}

impl GameObject {
    pub fn from_spawn(
        spawn: &GameObjectSpawnDbRecord,
        world_context: Arc<WorldContext>,
    ) -> Option<Self> {
        let data_store = world_context.data_store.clone();
        data_store
            .get_game_object_template(spawn.entry)
            .map(|template| {
                let guid =
                    ObjectGuid::with_entry(HighGuidType::Gameobject, spawn.entry, spawn.guid);
                let spawn_position = WorldPosition {
                    map_key: MapKey::for_continent(spawn.map), // TODO: MapKey for dungeon
                    zone: 0, // TODO: Calculate zone from terrain files
                    x: spawn.position_x,
                    y: spawn.position_y,
                    z: spawn.position_z,
                    o: spawn.orientation,
                };

                let mut values = InternalValues::new(GAME_OBJECT_END as usize);
                values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());

                let object_type = make_bitflags!(ObjectTypeMask::{Object | Gameobject}).bits();
                values.set_u32(ObjectFields::ObjectFieldType.into(), object_type);

                values.set_u32(ObjectFields::ObjectFieldEntry.into(), template.entry);
                values.set_f32(ObjectFields::ObjectFieldScaleX.into(), template.size);
                values.set_u32(
                    GameObjectFields::GameObjectDisplayid.into(),
                    template.display_id,
                );
                values.set_u8(GameObjectFields::GameObjectState.into(), 0, 1); // TODO: Enum GO_STATE
                values.set_u32(
                    GameObjectFields::GameObjectTypeId.into(),
                    template.go_type as u32,
                );
                values.set_u32(GameObjectFields::GameObjectAnimprogress.into(), 100); // FIXME: animprogress in DB

                values.set_f32(GameObjectFields::GameObjectPosX.into(), spawn.position_x);
                values.set_f32(GameObjectFields::GameObjectPosY.into(), spawn.position_y);
                values.set_f32(GameObjectFields::GameObjectPosZ.into(), spawn.position_z);

                values.set_f32(
                    GameObjectFields::GameObjectRotation.into(),
                    spawn.rotation.i,
                );
                values.set_f32(
                    GameObjectFields::GameObjectRotation as usize + 1,
                    spawn.rotation.j,
                );
                values.set_f32(
                    GameObjectFields::GameObjectRotation as usize + 2,
                    spawn.rotation.k,
                );
                values.set_f32(
                    GameObjectFields::GameObjectRotation as usize + 3,
                    spawn.rotation.w,
                );
                values.set_f32(
                    GameObjectFields::GameObjectFacing.into(),
                    spawn.get_orientation_from_rotation(),
                );

                values.set_u32(GameObjectFields::GameObjectFaction.into(), template.faction);
                values.set_u32(GameObjectFields::GameObjectFlags.into(), template.flags);

                GameObject {
                    guid,
                    relevant_quest_ids: template.quest_ids.clone(),
                    data_store: world_context.data_store.clone(),
                    internal_values: Arc::new(RwLock::new(values)),
                    spawn_position,
                }
            })
    }

    pub fn build_create_object_for(&self, player: &Player) -> SmsgCreateObject {
        let flags = make_bitflags!(UpdateFlag::{HighGuid | LowGuid | HasPosition});
        let mut update_builder = UpdateBlockBuilder::new();

        let internal_values = self.internal_values.read();
        for index in 0..GAME_OBJECT_END {
            let value = internal_values.get_u32(index as usize);
            if value != 0 {
                update_builder.add(index as usize, value);
            }
        }

        // Make the GameObject active and sparkling if player can interact with it (quest in the
        // appropriate status or quest that can be taken in case of a QuestGiver GO)
        for (quest_id, required_quest_status) in &self.relevant_quest_ids {
            match required_quest_status {
                PlayerQuestStatus::NotStarted => {
                    // Special case: here, the GameObject activates if the player can start the quest
                    let Some(quest_template) = self.data_store.get_quest_template(*quest_id) else {
                        continue;
                    };
                    Self::add_active_state_to_update(
                        &mut update_builder,
                        player.can_start_quest(quest_template),
                    );
                }
                _ => {
                    if let Some(quest_log_context) = player.quest_status(quest_id) {
                        let activate = self.should_activate(
                            *quest_id,
                            quest_log_context.status,
                            *required_quest_status,
                        );
                        Self::add_active_state_to_update(&mut update_builder, activate);
                    }
                }
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

    // Upon quest changes (accept, turn in, abandon, ...), GameObjects around a player might change
    // state, only for that player. For example, if they accept a quest, nearby Chest GameObjects
    // that loot the quest item must active (again, only for that player).
    pub fn build_update_for_quest(
        &self,
        quest_id: u32,
        quest_status: PlayerQuestStatus,
        player: &Player,
    ) -> Option<SmsgUpdateObject> {
        let (_, required_quest_status) = self
            .relevant_quest_ids
            .iter()
            .find(|(this_quest_id, _)| *this_quest_id == quest_id)?;

        let mut update_builder = UpdateBlockBuilder::new();

        match required_quest_status {
            PlayerQuestStatus::NotStarted => {
                // Special case: here, the GameObject activates if the player can start the quest
                if let Some(quest_template) = self.data_store.get_quest_template(quest_id) {
                    Self::add_active_state_to_update(
                        &mut update_builder,
                        player.can_start_quest(quest_template),
                    );
                }
            }
            _ => {
                let activate = self.should_activate(quest_id, quest_status, *required_quest_status);
                Self::add_active_state_to_update(&mut update_builder, activate);
            }
        }

        let blocks = update_builder.build();

        let update_data = vec![UpdateData {
            update_type: UpdateType::Values,
            packed_guid: self.guid.as_packed(),
            blocks,
        }];

        Some(SmsgUpdateObject {
            updates_count: update_data.len() as u32,
            has_transport: false,
            updates: update_data,
        })
    }

    fn should_activate(
        &self,
        _quest_id: u32,
        player_quest_status: PlayerQuestStatus,
        required_quest_status: PlayerQuestStatus,
    ) -> bool {
        player_quest_status == required_quest_status
    }

    fn add_active_state_to_update(update_builder: &mut UpdateBlockBuilder, activate: bool) {
        let flags = if activate {
            // FIXME: Spark is only for a few GO types (see Object::BuildValuesUpdate in MaNGOS)
            (GameObjectDynamicLowFlags::Activate | GameObjectDynamicLowFlags::Sparkle)
                .bits()
                .into()
        } else {
            0
        };
        update_builder.add(GameObjectFields::GameObjectDynFlags.into(), flags);
    }
}
