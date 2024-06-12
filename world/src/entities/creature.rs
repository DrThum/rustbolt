use std::sync::Arc;

use enumflags2::{make_bitflags, BitFlags};
use log::warn;
use parking_lot::{RwLock, RwLockWriteGuard};
use rand::{seq::SliceRandom, Rng};
use shipyard::Component;

use crate::{
    datastore::data_types::CreatureTemplate,
    ecs::components::movement::MovementKind,
    game::{loot::Loot, map_manager::MapKey, world_context::WorldContext},
    protocol::packets::SmsgCreateObject,
    repositories::creature::CreatureSpawnDbRecord,
    shared::constants::{
        CharacterClass, HighGuidType, NpcFlags, ObjectTypeId, ObjectTypeMask, PowerType,
        CREATURE_AGGRO_DISTANCE_AT_SAME_LEVEL, CREATURE_AGGRO_DISTANCE_MAX,
        CREATURE_AGGRO_DISTANCE_MIN, MAX_LEVEL_DIFFERENCE_FOR_AGGRO,
    },
    DataStore,
};

use super::{
    internal_values::InternalValues,
    object_guid::ObjectGuid,
    position::WorldPosition,
    update::{CreateData, MovementUpdateData, UpdateBlockBuilder, UpdateFlag, UpdateType},
    update_fields::{ObjectFields, UnitFields, UNIT_END},
};

#[derive(Component)]
pub struct Creature {
    data_store: Arc<DataStore>,
    guid: ObjectGuid,
    pub entry: u32,
    pub name: String,
    pub template: CreatureTemplate,
    pub spawn_position: WorldPosition,
    pub default_movement_kind: MovementKind,
    pub wander_radius: Option<u32>,
    pub npc_flags: BitFlags<NpcFlags>,
    pub internal_values: Arc<InternalValues>,
    loot: Arc<RwLock<Loot>>, // Reset on (re)spawn and generated on death
}

impl Creature {
    pub fn from_spawn(
        creature_spawn: &CreatureSpawnDbRecord,
        world_context: Arc<WorldContext>,
    ) -> Option<Self> {
        let data_store = world_context.data_store.clone();
        data_store
            .get_creature_template(creature_spawn.entry)
            .map(|template| {
                let mut rng = rand::thread_rng();

                let guid = ObjectGuid::with_entry(
                    HighGuidType::Unit,
                    creature_spawn.entry,
                    creature_spawn.guid,
                );
                let values = InternalValues::new(UNIT_END as usize);
                values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());

                let object_type = make_bitflags!(ObjectTypeMask::{Object | Unit}).bits();
                values.set_u32(ObjectFields::ObjectFieldType.into(), object_type);

                values.set_u32(ObjectFields::ObjectFieldEntry.into(), template.entry);

                values.set_f32(ObjectFields::ObjectFieldScaleX.into(), template.scale);

                values.set_u8(UnitFields::UnitFieldBytes0.into(), 1, template.unit_class as u8);

                let selected_level = rng.gen_range(template.min_level..=template.max_level);

                let creature_health = world_context
                    .data_store
                    .get_creature_base_attributes(template.unit_class, selected_level)
                    .map(|attrs| {
                            attrs.health(
                                template.expansion,
                                template.health_multiplier,
                            )
                    })
                    .expect("creature base attributes not found");

                // Set health
                values.set_u32(UnitFields::UnitFieldHealth.into(), creature_health);
                // FIXME: calculate max from base + modifiers
                values.set_u32(
                    UnitFields::UnitFieldMaxHealth.into(),
                            creature_health
                );

                // Set power type based on unit class
                match template.unit_class {
                    CharacterClass::Warrior => values.set_u8(UnitFields::UnitFieldBytes0.into(), 3, PowerType::Rage as u8),
                    CharacterClass::Rogue => values.set_u8(UnitFields::UnitFieldBytes0.into(), 3, PowerType::Energy as u8),
                    CharacterClass::Paladin | CharacterClass::Mage => {
                        values.set_u8(UnitFields::UnitFieldBytes0.into(), 3, PowerType::Mana as u8);
                        data_store.get_creature_base_attributes(template.unit_class, selected_level).map(|attrs| {
                            values.set_u32(UnitFields::UnitFieldBaseMana.into(), attrs.mana);
                            values.set_u32(UnitFields::UnitFieldPower1.into(), attrs.mana);
                            values.set_u32(UnitFields::UnitFieldMaxPower1.into(), attrs.mana);
                        });
                    },

                    _ => (),
                }

                values.set_u32(
                    UnitFields::UnitFieldLevel.into(),
                    selected_level
                );

                let existing_model_ids: Vec<&u32> =
                    template.model_ids.iter().filter(|&&id| id != 0).collect();
                let display_id = existing_model_ids.choose(&mut rng).expect("rng error");
                values.set_u32(UnitFields::UnitFieldDisplayid.into(), **display_id);
                values.set_u32(UnitFields::UnitFieldNativedisplayid.into(), **display_id);
                let model_info = data_store
                    .get_creature_model_info(**display_id)
                    .expect(format!("creature entry {} has invalid model id {}", template.entry, **display_id).as_str());
                values.set_f32(UnitFields::UnitFieldCombatReach.into(), model_info.combat_reach);
                values.set_f32(UnitFields::UnitFieldBoundingRadius.into(), template.scale * model_info.bounding_radius);

                values.set_u32(
                    UnitFields::UnitFieldFactionTemplate.into(),
                    template.faction_template_id,
                );

                values.set_u32(UnitFields::UnitNpcFlags.into(), template.npc_flags);
                values.set_u32(UnitFields::UnitFieldFlags.into(), template.unit_flags);
                values.set_u32(UnitFields::UnitDynamicFlags.into(), template.dynamic_flags);

                let mut default_movement_kind = creature_spawn
                    .movement_type_override
                    .unwrap_or(template.movement_type);
                let wander_radius = creature_spawn.wander_radius;

                if wander_radius.is_none() && default_movement_kind.is_random() {
                    warn!(
                        "creature spawn with guid {} has random movement but no wander radius - defaulting to idle movement",
                        guid.counter()
                    );
                    default_movement_kind = MovementKind::Idle;
                }

                Creature {
                    data_store: data_store.clone(),
                    guid,
                    entry: template.entry,
                    name: template.name.to_owned(),
                    template: template.clone(),
                    spawn_position: WorldPosition {
                        map_key: MapKey::for_continent(creature_spawn.map), // TODO: MapKey for dungeon
                        zone: 0, // TODO: Calculate zone from terrain files
                        x: creature_spawn.position_x,
                        y: creature_spawn.position_y,
                        z: creature_spawn.position_z,
                        o: creature_spawn.orientation,
                    },
                    npc_flags: unsafe { BitFlags::from_bits_unchecked(template.npc_flags) },
                    internal_values: Arc::new(values),
                    default_movement_kind,
                    wander_radius,
                    loot: Arc::new(RwLock::new(Loot::new())),
                }
            })
    }

    pub fn build_create_object(&self, movement: Option<MovementUpdateData>) -> SmsgCreateObject {
        let flags = make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition});
        let mut update_builder = UpdateBlockBuilder::new();

        for index in 0..UNIT_END {
            let value = self.internal_values.get_u32(index as usize);
            if value != 0 {
                update_builder.add(index as usize, value);
            }
        }

        let blocks = update_builder.build();

        let update_data = vec![CreateData {
            update_type: UpdateType::CreateObject2,
            packed_guid: self.guid.as_packed(),
            object_type: ObjectTypeId::Unit,
            flags,
            movement,
            low_guid_part: None,
            high_guid_part: Some(HighGuidType::Unit as u32),
            blocks,
        }];

        SmsgCreateObject {
            updates_count: update_data.len() as u32,
            has_transport: false,
            updates: update_data,
        }
    }

    // The maximum Aggro Radius has a cap of 25 levels under
    // Example: A level 30 char has the same Aggro Radius than a level 5 char on a level 60 mob
    // The aggro radius of a mob having the same level as the player is roughly 20 yards
    // Aggro Radius varies with level difference at the rate of roughly 1 yard/level
    // and radius grows if player level < creature level
    // Minimum Aggro Radius for a mob seems to be combat range (5 yards)
    pub fn aggro_distance(&self, other_entity_level: u32) -> f32 {
        let level_difference: i32 = (other_entity_level as i32
            - self.level_against(other_entity_level) as i32)
            .max(MAX_LEVEL_DIFFERENCE_FOR_AGGRO);

        let aggro_distance: f32 = CREATURE_AGGRO_DISTANCE_AT_SAME_LEVEL - level_difference as f32;

        // TODO: Handle aura type SPELL_AURA_MOD_DETECT_RANGE
        aggro_distance.clamp(CREATURE_AGGRO_DISTANCE_MIN, CREATURE_AGGRO_DISTANCE_MAX)
    }

    pub fn level_against(&self, _other_entity_level: u32) -> u32 {
        // TODO: World Boss case, need other_entity_level
        self.internal_values
            .get_u32(UnitFields::UnitFieldLevel.into())
    }

    pub fn real_level(&self) -> u32 {
        self.internal_values
            .get_u32(UnitFields::UnitFieldLevel.into())
    }

    pub fn guid(&self) -> &ObjectGuid {
        &self.guid
    }

    // Returns whether we actually generated some loots
    pub fn generate_loot(&self) -> bool {
        let mut loot = Loot::new();
        loot.add_money(self.template.min_money_loot, self.template.max_money_loot);

        if let Some(loot_table) = self
            .template
            .loot_table_id
            .and_then(|loot_table_id| self.data_store.get_creature_loot_table(loot_table_id))
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

    pub fn remove_loot_money(&self) -> u32 {
        let loot = &mut *self.loot.write();

        if loot.money() > 0 {
            let money = loot.money();
            loot.remove_money();
            money
        } else {
            warn!("attempt to remove loot money from a creature with generated loot but no money in it");
            0
        }
    }
}
