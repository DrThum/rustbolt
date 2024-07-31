use std::sync::Arc;

use enumflags2::make_bitflags;
use log::warn;
use parking_lot::{RwLock, RwLockWriteGuard};
use shipyard::Component;

use crate::{
    datastore::data_types::{GameObjectTemplate, QuestTemplate},
    game::{loot::Loot, map_manager::MapKey, world_context::WorldContext},
    protocol::packets::{SmsgCreateObject, SmsgUpdateObject},
    repositories::game_object::GameObjectSpawnDbRecord,
    shared::constants::{
        GameObjectDynamicLowFlags, HighGuidType, ObjectTypeId, ObjectTypeMask, PlayerQuestStatus,
        MAX_QUEST_OBJECTIVES_COUNT,
    },
    DataStore,
};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    player::{player_data::QuestLogContext, Player},
    position::WorldPosition,
    update::{
        CreateData, PositionUpdateData, UpdateBlockBuilder, UpdateData, UpdateFlag, UpdateType,
    },
    update_fields::{GameObjectFields, ObjectFields, GAME_OBJECT_END},
};

#[derive(Component)]
pub struct GameObject {
    guid: ObjectGuid,
    template: GameObjectTemplate,
    data_store: Arc<DataStore>,
    pub internal_values: Arc<RwLock<InternalValues>>,
    pub spawn_position: WorldPosition,
    loot: Arc<RwLock<Loot>>,
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
                    template: template.clone(),
                    data_store: world_context.data_store.clone(),
                    internal_values: Arc::new(RwLock::new(values)),
                    spawn_position,
                    loot: Arc::new(RwLock::new(Loot::new())),
                }
            })
    }

    pub fn guid(&self) -> ObjectGuid {
        self.guid
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

        if self.should_activate_for_player(player, None) {
            Self::add_active_state_to_update(&mut update_builder, true);
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
        _quest_log_context: QuestLogContext,
        player: &Player,
    ) -> Option<SmsgUpdateObject> {
        let mut update_builder = UpdateBlockBuilder::new();
        Self::add_active_state_to_update(
            &mut update_builder,
            self.should_activate_for_player(player, Some(quest_id)),
        );

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

    fn should_activate_for_player(&self, player: &Player, specific_quest_id: Option<u32>) -> bool {
        fn should_activate_for_in_progress_quest(
            data_store: Arc<DataStore>,
            game_object_template: &GameObjectTemplate,
            quest_template: &QuestTemplate,
            quest_log_context: &QuestLogContext,
            player: &Player,
        ) -> bool {
            // Activate if the player still needs required entity from the template...
            for i in 0..MAX_QUEST_OBJECTIVES_COUNT {
                if quest_template.required_entity_ids[i] < 0
                    && (-quest_template.required_entity_ids[i] as u32) == game_object_template.entry
                    && quest_template.required_entity_counts[i] < quest_log_context.entity_counts[i]
                {
                    return true;
                }
            }
            // ...Or if the GameObject can drop an item that the player needs for the quest
            if let Some(loot_table) = game_object_template
                .loot_table_id()
                .and_then(|loot_table_id| data_store.get_loot_table(loot_table_id))
            {
                let all_possible_loot_ids = loot_table.get_all_possible_item_ids();

                for i in 0..MAX_QUEST_OBJECTIVES_COUNT {
                    let required_item_id = quest_template.required_item_ids[i];
                    if all_possible_loot_ids.contains(&required_item_id)
                        && player.inventory().get_item_count(required_item_id)
                            < quest_template.required_item_counts[i]
                    {
                        return true;
                    }
                }
            }

            false
        }

        // Make the GameObject active and sparkling if player can interact with it (quest in the
        // appropriate status or quest that can be taken in case of a QuestGiver GO)
        for (quest_id, required_quest_status) in &self.template.quest_ids {
            let Some(quest_template) = self.data_store.get_quest_template(*quest_id) else {
                continue;
            };

            match required_quest_status {
                PlayerQuestStatus::NotStarted => {
                    if player.can_start_quest(quest_template) {
                        return true;
                    }
                }
                PlayerQuestStatus::ObjectivesCompleted => {
                    if let Some(quest_log_context) = player.quest_status(quest_id) {
                        if quest_log_context.status == PlayerQuestStatus::ObjectivesCompleted {
                            return true;
                        }
                    }
                }
                PlayerQuestStatus::InProgress => {
                    if let Some(quest_log_context) = player.quest_status(quest_id) {
                        if should_activate_for_in_progress_quest(
                            self.data_store.clone(),
                            &self.template,
                            quest_template,
                            quest_log_context,
                            player,
                        ) {
                            return true;
                        }
                    }
                }
                status => warn!(
                    "status {status:?} not implemented in GameObject::should_activate_for_player"
                ),
            }
        }

        // Check either the specific quest or all of player's journal quests
        let quests_to_check: Vec<u32> = specific_quest_id
            .map(|quest_id| vec![quest_id])
            .unwrap_or(player.get_active_quest_ids());

        for quest_id in quests_to_check {
            let Some(quest_template) = self.data_store.get_quest_template(quest_id) else {
                continue;
            };

            let Some(quest_log_context) = player.quest_status(&quest_id) else {
                continue;
            };

            match quest_log_context.status {
                PlayerQuestStatus::InProgress => {
                    if should_activate_for_in_progress_quest(
                        self.data_store.clone(),
                        &self.template,
                        quest_template,
                        quest_log_context,
                        player,
                    ) {
                        return true;
                    }
                }
                _ => continue,
            }
        }

        false
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

    pub fn generate_loot(&self, replace_if_loot_non_empty: bool) -> bool {
        if !self.loot().is_empty() && !replace_if_loot_non_empty {
            return true;
        }

        let mut loot = Loot::new();

        if let Some(loot_table) = self
            .template
            .loot_table_id()
            .and_then(|loot_table_id| self.data_store.get_loot_table(loot_table_id))
        {
            let items = loot_table.generate_loots();
            for item in items {
                loot.add_item(item.item_id, item.count.random_value().into())
            }
        }

        let has_loot = !loot.is_empty();
        *self.loot.write() = loot;
        has_loot
    }

    pub fn loot(&self) -> Loot {
        self.loot.read().clone()
    }

    pub fn loot_mut(&self) -> RwLockWriteGuard<Loot> {
        self.loot.write()
    }
}
