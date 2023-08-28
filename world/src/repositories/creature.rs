use std::time::Duration;

use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{
    named_params,
    types::{FromSql, FromSqlError},
};

use crate::{
    datastore::data_types::CreatureTemplate,
    ecs::components::movement::MovementKind,
    shared::constants::{Gender, MAX_CREATURE_TEMPLATE_MODELID},
};

pub struct CreatureRepository;

impl CreatureRepository {
    pub fn load_templates(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<CreatureTemplate> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(entry) FROM creature_templates")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare_cached("SELECT entry, name, sub_name, icon_name, min_level, max_level, model_id1, model_id2, model_id3, model_id4, scale, family, type_id, racial_leader, type_flags, speed_walk, speed_run, rank, health_multiplier, power_multiplier, min_level_health, max_level_health, min_level_mana, max_level_mana, melee_base_attack_time_ms, ranged_base_attack_time_ms, pet_spell_data_id, faction_template_id, npc_flags, unit_flags, dynamic_flags, gossip_menu_id, movement_type FROM creature_templates ORDER BY entry").unwrap();

        let result = stmt
            .query_map([], |row| {
                use CreatureTemplateColumnIndex::*;

                let model_ids: Vec<u32> = (0..MAX_CREATURE_TEMPLATE_MODELID)
                    .into_iter()
                    .map(|index| row.get(ModelId1 as usize + index).unwrap())
                    .collect();

                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(CreatureTemplate {
                    entry: row.get(Entry as usize).unwrap(),
                    name: row.get(Name as usize).unwrap(),
                    sub_name: row.get(SubName as usize).unwrap(),
                    icon_name: row.get(IconName as usize).unwrap(),
                    min_level: row.get(MinLevel as usize).unwrap(),
                    max_level: row.get(MaxLevel as usize).unwrap(),
                    min_level_health: row.get(MinLevelHealth as usize).unwrap(),
                    max_level_health: row.get(MaxLevelHealth as usize).unwrap(),
                    min_level_mana: row.get(MinLevelMana as usize).unwrap(),
                    max_level_mana: row.get(MaxLevelMana as usize).unwrap(),
                    melee_base_attack_time: Duration::from_millis(
                        row.get::<usize, u64>(MeleeBaseAttackTimeMs as usize)
                            .unwrap(),
                    ),
                    ranged_base_attack_time: Duration::from_millis(
                        row.get::<usize, u64>(RangedBaseAttackTimeMs as usize)
                            .unwrap(),
                    ),
                    model_ids,
                    scale: row.get(Scale as usize).unwrap(),
                    speed_walk: row.get(SpeedWalk as usize).unwrap(),
                    speed_run: row.get(SpeedRun as usize).unwrap(),
                    family: row.get(Family as usize).unwrap(),
                    type_id: row.get(TypeId as usize).unwrap(),
                    type_flags: row.get(TypeFlags as usize).unwrap(),
                    rank: row.get(Rank as usize).unwrap(),
                    racial_leader: row.get(RacialLeader as usize).unwrap(),
                    health_multiplier: row.get(HealthMultiplier as usize).unwrap(),
                    power_multiplier: row.get(PowerMultiplier as usize).unwrap(),
                    pet_spell_data_id: row.get(PetSpellDataId as usize).unwrap(),
                    faction_template_id: row.get(FactionTemplateId as usize).unwrap(),
                    npc_flags: row.get(NpcFlags as usize).unwrap(),
                    unit_flags: row.get(UnitFlags as usize).unwrap(),
                    dynamic_flags: row.get(DynamicFlags as usize).unwrap(),
                    gossip_menu_id: row.get(GossipMenuId as usize).unwrap(),
                    movement_type: row.get(MovementType as usize).unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }

    pub fn load_creature_spawns(
        conn: &PooledConnection<SqliteConnectionManager>,
        map_id: u32,
    ) -> Vec<CreatureSpawnDbRecord> {
        let mut stmt = conn.prepare_cached("SELECT guid, entry, map, position_x, position_y, position_z, orientation, movement_type_override, wander_radius FROM creature_spawns WHERE map = :map_id").unwrap();

        let result = stmt
            .query_map(named_params! { ":map_id": map_id }, |row| {
                use CreatureSpawnColumnIndex::*;

                Ok(CreatureSpawnDbRecord {
                    guid: row.get(Guid as usize).unwrap(),
                    entry: row.get(Entry as usize).unwrap(),
                    map: row.get(Map as usize).unwrap(),
                    position_x: row.get(PositionX as usize).unwrap(),
                    position_y: row.get(PositionY as usize).unwrap(),
                    position_z: row.get(PositionZ as usize).unwrap(),
                    orientation: row.get(Orientation as usize).unwrap(),
                    movement_type_override: row.get(MovementTypeOverride as usize).unwrap(),
                    wander_radius: row.get(WanderRadius as usize).unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }

    pub fn load_creature_model_info(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<CreatureModelInfo> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(model_id) FROM creature_model_info")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare_cached("SELECT model_id, bounding_radius, combat_reach, gender, model_id_other_gender, model_id_alternative FROM creature_model_info").unwrap();

        let result = stmt
            .query_map([], |row| {
                use CreatureModelInfoColumnIndex::*;

                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(CreatureModelInfo {
                    model_id: row.get(ModelId as usize).unwrap(),
                    bounding_radius: row.get(BoundingRadius as usize).unwrap(),
                    combat_reach: row.get(CombatReach as usize).unwrap(),
                    gender: row.get(Gender as usize).unwrap(),
                    model_id_other_gender: row.get(ModelIdOtherGender as usize).unwrap(),
                    model_id_alternative: row.get(ModelIdAlternative as usize).unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }
}

pub struct CreatureSpawnDbRecord {
    pub guid: u32,
    pub entry: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub movement_type_override: Option<MovementKind>,
    pub wander_radius: Option<u32>,
}

#[allow(dead_code)]
enum CreatureTemplateColumnIndex {
    Entry,
    Name,
    SubName,
    IconName,
    MinLevel,
    MaxLevel,
    ModelId1,
    ModelId2,
    ModelId3,
    ModelId4,
    Scale,
    Family,
    TypeId,
    RacialLeader,
    TypeFlags,
    SpeedWalk,
    SpeedRun,
    Rank,
    HealthMultiplier,
    PowerMultiplier,
    MinLevelHealth,
    MaxLevelHealth,
    MinLevelMana,
    MaxLevelMana,
    MeleeBaseAttackTimeMs,
    RangedBaseAttackTimeMs,
    PetSpellDataId,
    FactionTemplateId,
    NpcFlags,
    UnitFlags,
    DynamicFlags,
    GossipMenuId,
    MovementType,
}

enum CreatureSpawnColumnIndex {
    Guid,
    Entry,
    Map,
    PositionX,
    PositionY,
    PositionZ,
    Orientation,
    MovementTypeOverride,
    WanderRadius,
}

pub struct CreatureModelInfo {
    pub model_id: u32,
    pub bounding_radius: f32,
    pub combat_reach: f32,
    pub gender: Gender,
    pub model_id_other_gender: u32,
    pub model_id_alternative: u32,
}

enum CreatureModelInfoColumnIndex {
    ModelId,
    BoundingRadius,
    CombatReach,
    Gender,
    ModelIdOtherGender,
    ModelIdAlternative,
}

impl FromSql for Gender {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = value.as_i64()?;
        Gender::n(value).map_or(Err(FromSqlError::Other("invalid gender".into())), Ok)
    }
}
