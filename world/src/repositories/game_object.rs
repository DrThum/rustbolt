use std::f32::consts;

use indicatif::ProgressBar;
use parry3d::na::Quaternion;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::named_params;

use crate::{
    datastore::{
        data_types::{GameObjectData, GameObjectTemplate, QuestRelation, QuestTemplate},
        SqlMultiStore, SqlStore,
    },
    shared::constants::GameObjectType,
};

pub struct GameObjectRepository;

impl GameObjectRepository {
    pub fn load_templates(
        conn: &PooledConnection<SqliteConnectionManager>,
        quest_templates_store: &SqlStore<QuestTemplate>,
        quest_relations_by_game_object: &SqlMultiStore<QuestRelation>,
    ) -> Vec<GameObjectTemplate> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(entry) FROM game_object_templates")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn
            .prepare_cached(
                "
            SELECT entry, type, display_id, name, cast_bar_caption, faction, flags, size,
                data0, data1, data2, data3, data4, data5, data6, data7, data8, data9, data10,
                data11, data12, data13, data14, data15, data16, data17, data18, data19, data20,
                data21, data22, data23
            FROM game_object_templates
            ORDER BY entry
        ",
            )
            .unwrap();

        let result = stmt
            .query_map([], |row| {
                use GameObjectTemplateColumnIndex::*;

                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                let go_type: u32 = row.get(Type as usize).unwrap();
                let go_type = GameObjectType::n(go_type).unwrap();

                let raw_data = [
                    row.get(Data0 as usize).unwrap(),
                    row.get(Data1 as usize).unwrap(),
                    row.get(Data2 as usize).unwrap(),
                    row.get(Data3 as usize).unwrap(),
                    row.get(Data4 as usize).unwrap(),
                    row.get(Data5 as usize).unwrap(),
                    row.get(Data6 as usize).unwrap(),
                    row.get(Data7 as usize).unwrap(),
                    row.get(Data8 as usize).unwrap(),
                    row.get(Data9 as usize).unwrap(),
                    row.get(Data10 as usize).unwrap(),
                    row.get(Data11 as usize).unwrap(),
                    row.get(Data12 as usize).unwrap(),
                    row.get(Data13 as usize).unwrap(),
                    row.get(Data14 as usize).unwrap(),
                    row.get(Data15 as usize).unwrap(),
                    row.get(Data16 as usize).unwrap(),
                    row.get(Data17 as usize).unwrap(),
                    row.get(Data18 as usize).unwrap(),
                    row.get(Data19 as usize).unwrap(),
                    row.get(Data20 as usize).unwrap(),
                    row.get(Data21 as usize).unwrap(),
                    row.get(Data22 as usize).unwrap(),
                    row.get(Data23 as usize).unwrap(),
                ];

                let mut template = GameObjectTemplate {
                    entry: row.get(Entry as usize).unwrap(),
                    go_type,
                    display_id: row.get(DisplayId as usize).unwrap(),
                    name: row.get(Name as usize).unwrap(),
                    cast_bar_caption: row.get(CastBarCaption as usize).unwrap(),
                    faction: row.get(Faction as usize).unwrap(),
                    flags: row.get(Flags as usize).unwrap(),
                    size: row.get(Size as usize).unwrap(),
                    data: Self::build_template_data(go_type, raw_data),
                    raw_data,
                    quest_ids: vec![],
                };

                template.initialize_relevant_quests(
                    quest_templates_store,
                    quest_relations_by_game_object,
                );

                Ok(template)
            })
            .unwrap();

        result.filter_map(|res| res.ok()).collect()
    }

    pub fn load_game_object_spawns(
        conn: &PooledConnection<SqliteConnectionManager>,
        map_id: u32,
    ) -> Vec<GameObjectSpawnDbRecord> {
        let mut stmt = conn.prepare_cached("
            SELECT guid, entry, map, position_x, position_y, position_z, orientation, rotation0, rotation1, rotation2, rotation3
            FROM game_object_spawns WHERE map = :map_id").unwrap();

        let result = stmt
            .query_map(named_params! { ":map_id": map_id }, |row| {
                use GameObjectSpawnColumnIndex::*;

                let rotation0: f32 = row.get(Rotation0 as usize).unwrap();
                let rotation1: f32 = row.get(Rotation1 as usize).unwrap();
                let rotation2: f32 = row.get(Rotation2 as usize).unwrap();
                let rotation3: f32 = row.get(Rotation3 as usize).unwrap();

                Ok(GameObjectSpawnDbRecord {
                    guid: row.get(Guid as usize).unwrap(),
                    entry: row.get(Entry as usize).unwrap(),
                    map: row.get(Map as usize).unwrap(),
                    position_x: row.get(PositionX as usize).unwrap(),
                    position_y: row.get(PositionY as usize).unwrap(),
                    position_z: row.get(PositionZ as usize).unwrap(),
                    orientation: row.get(Orientation as usize).unwrap(),
                    rotation: Quaternion::new(rotation3, rotation0, rotation1, rotation2)
                        .normalize(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).collect()
    }

    fn build_template_data(go_type: GameObjectType, raw_data: [u32; 24]) -> GameObjectData {
        use GameObjectType::*;

        match go_type {
            Door => GameObjectData::Door {
                isStartOpen: raw_data[0] != 0,
                openLockId: raw_data[1],
                autoCloseTimerSecs: raw_data[2],
                isNoDamageImmune: raw_data[3] != 0,
                openTextId: raw_data[4],
                closeTextId: raw_data[5],
            },
            Button => GameObjectData::Button {
                isStartOpen: raw_data[0] != 0,
                openLockId: raw_data[1],
                autoCloseTimerSecs: raw_data[2],
                linkedTrapGameObjectEntry: raw_data[3],
                isNoDamageImmune: raw_data[4] != 0,
                isLarge: raw_data[5] != 0,
                openTextId: raw_data[6],
                closeTextId: raw_data[7],
                isLineOfSightOK: raw_data[8] != 0,
            },
            QuestGiver => GameObjectData::QuestGiver {
                openLockId: raw_data[0],
                questList: raw_data[1],
                pageMaterialId: raw_data[2],
                gossipId: raw_data[3],
                customAnim: raw_data[4],
                isNoDamageImmune: raw_data[5] != 0,
                openTextId: raw_data[6],
                isLineOfSightOK: raw_data[7] != 0,
                doesAllowMounted: raw_data[8] != 0,
                isLarge: raw_data[9] != 0,
            },
            Chest => GameObjectData::Chest {
                openLockId: raw_data[0],
                lootTemplateEntry: raw_data[1],
                restockTimerSecs: raw_data[2],
                isConsumable: raw_data[3] != 0,
                minLootAttempt: raw_data[4],
                maxLootAttempt: raw_data[5],
                lootedEventId: raw_data[6],
                linkedTrapGameObjectEntry: raw_data[7],
                questId: raw_data[8],
                minLevelToOpen: raw_data[9],
                isLineOfSightOK: raw_data[10] != 0,
                isLeaveLoot: raw_data[11] != 0,
                notInCombat: raw_data[12] != 0,
                shouldLogLoot: raw_data[13] != 0,
                openTextId: raw_data[14],
                usesGroupLootRules: raw_data[15] != 0,
            },
            Binder => GameObjectData::Binder,
            Generic => GameObjectData::Generic {
                isFloatingTooltip: raw_data[0] != 0,
                isHighlighted: raw_data[1] != 0,
                isServerOnly: raw_data[2] != 0,
                isLarge: raw_data[3] != 0,
                isFloatingOnWater: raw_data[4] != 0,
                questId: raw_data[5],
            },
            Trap => GameObjectData::Trap {
                openLockId: raw_data[0],
                level: raw_data[1],
                diameter: raw_data[2],
                spellId: raw_data[3],
                charges: raw_data[4],
                cooldownSecs: raw_data[5],
                isAutoClose: raw_data[6] != 0,
                startDelaySecs: raw_data[7],
                isServerOnly: raw_data[8] != 0,
                isStealthed: raw_data[9] != 0,
                isLarge: raw_data[10] != 0,
                isAffectedByStealth: raw_data[11] != 0,
                openTextId: raw_data[12],
            },
            Chair => GameObjectData::Chest {
                openLockId: raw_data[0],
                lootTemplateEntry: raw_data[1],
                restockTimerSecs: raw_data[2],
                isConsumable: raw_data[3] != 0,
                minLootAttempt: raw_data[4],
                maxLootAttempt: raw_data[5],
                lootedEventId: raw_data[6],
                linkedTrapGameObjectEntry: raw_data[7],
                questId: raw_data[8],
                minLevelToOpen: raw_data[9],
                isLineOfSightOK: raw_data[10] != 0,
                isLeaveLoot: raw_data[11] != 0,
                notInCombat: raw_data[12] != 0,
                shouldLogLoot: raw_data[13] != 0,
                openTextId: raw_data[14],
                usesGroupLootRules: raw_data[15] != 0,
            },
            SpellFocus => GameObjectData::SpellFocus {
                spellFocusType: raw_data[0],
                diameter: raw_data[1],
                linkedTrapGameObjectEntry: raw_data[2],
                isServerOnly: raw_data[3] != 0,
                questId: raw_data[4],
                isLarge: raw_data[5] != 0,
            },
            Text => GameObjectData::Text {
                pageId: raw_data[0],
                languageId: raw_data[1],
                pageMaterialId: raw_data[2],
            },
            Goober => GameObjectData::Goober {
                openLockId: raw_data[0],
                questId: raw_data[1],
                eventId: raw_data[2],
                isAutoClose: raw_data[3] != 0,
                customAnim: raw_data[4],
                isConsumable: raw_data[5] != 0,
                cooldownSecs: raw_data[6],
                pageId: raw_data[7],
                languageId: raw_data[8],
                pageMaterialId: raw_data[9],
                spellId: raw_data[10],
                isNoDamageImmune: raw_data[11] != 0,
                linkedTrapGameObjectEntry: raw_data[12],
                isLarge: raw_data[13] != 0,
                openTextId: raw_data[14],
                closeTextId: raw_data[15],
                isLineOfSightOK: raw_data[16] != 0,
            },
            Transport => GameObjectData::Transport,
            AreaDamage => GameObjectData::AreaDamage,
            Camera => GameObjectData::Camera {
                openLockId: raw_data[0],
                cinematicId: raw_data[1],
            },
            MapObject => GameObjectData::MapObject,
            MoTransport => GameObjectData::MoTransport {
                taxiPathId: raw_data[0],
                moveSpeed: raw_data[1],
                accelRate: raw_data[2],
            },
            DuelArbiter => GameObjectData::DualArbiter,
            FishingNode => GameObjectData::FishingNode,
            SummoningRitual => GameObjectData::SummoningRitual {
                casters: raw_data[0],
                spellId: raw_data[1],
                animSpell: raw_data[2],
                isPersistent: raw_data[3] != 0,
                casterTargetSpell: raw_data[4],
                isCasterTargetSpellTargets: raw_data[5] != 0,
                areCastersGrouped: raw_data[6] != 0,
            },
            MailBox => GameObjectData::MailBox,
            AuctionHouse => GameObjectData::AuctionHouse {
                auctionHouseId: raw_data[0],
            },
            GuardPost => GameObjectData::GuardPost,
            SpellCaster => GameObjectData::SpellCaster {
                spellId: raw_data[0],
                charges: raw_data[1],
                isPartyOnly: raw_data[2] != 0,
            },
            MeetingStone => GameObjectData::MeetingStone {
                minLevel: raw_data[0],
                maxLevel: raw_data[1],
                areaId: raw_data[2],
            },
            FlagStand => GameObjectData::FlagStand {
                openLockId: raw_data[0],
                pickupSpellId: raw_data[1],
                radius: raw_data[2],
                returnAuraId: raw_data[3],
                returnSpellId: raw_data[4],
                isNoDamageImmune: raw_data[5] != 0,
                openTextId: raw_data[6],
                isLineOfSightOK: raw_data[7] != 0,
            },
            FishingHole => GameObjectData::FishingHole {
                radius: raw_data[0],
                lootTemplateEntry: raw_data[1],
                minLootAttempt: raw_data[2],
                maxLootAttempt: raw_data[3],
            },
            FlagDrop => GameObjectData::FlagDrop {
                openLockId: raw_data[0],
                eventId: raw_data[1],
                pickupSpellId: raw_data[2],
                isNoDamageImmune: raw_data[3] != 0,
            },
            MiniGame => GameObjectData::MiniGame,
            LotteryKiosk => GameObjectData::LotteryKiosk,
            CapturePoint => GameObjectData::CapturePoint {
                radius: raw_data[0],
                spellId: raw_data[1],
                worldState1: raw_data[2],
                worldState2: raw_data[3],
                winEventId1: raw_data[4],
                winEventId2: raw_data[5],
                contestedEventId1: raw_data[6],
                contestedEventId2: raw_data[7],
                progressEventId1: raw_data[8],
                progressEventId2: raw_data[9],
                neutralEventId1: raw_data[10],
                neutralEventId2: raw_data[11],
                neutralPercent: raw_data[12],
                worldState3: raw_data[13],
                minSuperiority: raw_data[14],
                maxSuperiority: raw_data[15],
                minTimeSecs: raw_data[16],
                maxTimeSecs: raw_data[17],
                isLarge: raw_data[18] != 0,
            },
            AuraGenerator => GameObjectData::AuraGenerator {
                isStartOpen: raw_data[0] != 0,
                radius: raw_data[1],
                auraId: raw_data[2],
                conditionId: raw_data[3],
            },
            DungeonDifficulty => GameObjectData::DungeonDifficulty {
                mapId: raw_data[0],
                difficulty: raw_data[1],
            },
            BarberChair => GameObjectData::BarberChair,
            DestructibleBuilding => GameObjectData::DestructibleBuilding,
            GuildBank => GameObjectData::GuildBank,
        }
    }
}

pub struct GameObjectSpawnDbRecord {
    pub guid: u32,
    pub entry: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub rotation: Quaternion<f32>,
}

impl GameObjectSpawnDbRecord {
    // I have no idea what I'm doing, this is straight from MaNGOS
    pub fn get_orientation_from_rotation(&self) -> f32 {
        let q = self.rotation;
        let t1 = 2. * (q.w * q.k + q.i * q.j);
        let t2 = 1. - 2. * (q.j * q.j + q.k * q.k);
        let orientation = f32::atan2(t1, t2);
        orientation % (2. * consts::PI)
    }
}

enum GameObjectTemplateColumnIndex {
    Entry,
    Type,
    DisplayId,
    Name,
    CastBarCaption,
    Faction,
    Flags,
    Size,
    Data0,
    Data1,
    Data2,
    Data3,
    Data4,
    Data5,
    Data6,
    Data7,
    Data8,
    Data9,
    Data10,
    Data11,
    Data12,
    Data13,
    Data14,
    Data15,
    Data16,
    Data17,
    Data18,
    Data19,
    Data20,
    Data21,
    Data22,
    Data23,
}

enum GameObjectSpawnColumnIndex {
    Guid,
    Entry,
    Map,
    PositionX,
    PositionY,
    PositionZ,
    Orientation,
    Rotation0,
    Rotation1,
    Rotation2,
    Rotation3,
}
