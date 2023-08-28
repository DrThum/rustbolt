use std::{collections::HashMap, sync::Arc};

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, Error, Transaction};

use crate::{
    datastore::{
        data_types::{ItemRecord, PlayerCreatePosition},
        DataStore,
    },
    ecs::components::health::Health,
    entities::{
        player::{
            player_data::{ActionButton, CharacterSkill, QuestLogContext},
            Player, PlayerVisualFeatures,
        },
        position::WorldPosition,
    },
    game::map_manager::MapKey,
    protocol::packets::{CharEnumData, CharEnumEquip, CmsgCharCreate, CmsgCharDelete},
    shared::constants::{
        ActionButtonType, CharacterClass, CharacterRace, InventorySlot, InventoryType,
        PlayerQuestStatus, MAX_QUEST_OBJECTIVES_COUNT,
    },
};

pub struct CharacterRepository;

impl CharacterRepository {
    pub fn is_name_available(
        conn: &PooledConnection<SqliteConnectionManager>,
        name: String,
    ) -> bool {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(guid) FROM characters WHERE name = :name")
            .unwrap();
        let mut count = stmt
            .query_map(named_params! { ":name": name }, |row| {
                row.get::<usize, u32>(0)
            })
            .unwrap();

        count.next().unwrap().map(|c| c == 0).unwrap_or(true)
    }

    pub fn create_character(
        transaction: &Transaction,
        source: &CmsgCharCreate,
        account_id: u32,
        create_position: &PlayerCreatePosition,
    ) -> u64 {
        let mut stmt_create = transaction.prepare_cached("INSERT INTO characters (guid, account_id, name, race, class, gender, skin, face, hairstyle, haircolor, facialstyle, map_id, zone_id, position_x, position_y, position_z, orientation) VALUES (NULL, :account_id, :name, :race, :class, :gender, :skin, :face, :hairstyle, :haircolor, :facialstyle, :map, :zone, :x, :y, :z, :o)").unwrap();
        stmt_create
            .execute(named_params! {
                ":account_id": account_id,
                ":name": source.name.to_string(),
                ":race": source.race,
                ":class": source.class,
                ":gender": source.gender,
                ":skin": source.skin,
                ":face": source.face,
                ":hairstyle": source.hairstyle,
                ":haircolor": source.haircolor,
                ":facialstyle": source.facialstyle,
                ":map": create_position.map,
                ":zone": create_position.zone,
                ":x": create_position.x,
                ":y": create_position.y,
                ":z": create_position.z,
                ":o": create_position.o,
            })
            .unwrap();

        transaction.last_insert_rowid() as u64
    }

    // Note: don't use it for anything else than char enum, it's incomplete
    fn to_inventory_type_for_enum(inv_slot: &InventorySlot) -> InventoryType {
        match inv_slot {
            InventorySlot::EquipmentHead => InventoryType::Head,
            InventorySlot::EquipmentNeck => InventoryType::Neck,
            InventorySlot::EquipmentShoulders => InventoryType::Shoulders,
            InventorySlot::EquipmentBody => InventoryType::Body,
            InventorySlot::EquipmentChest => InventoryType::Chest,
            InventorySlot::EquipmentWaist => InventoryType::Waist,
            InventorySlot::EquipmentLegs => InventoryType::Legs,
            InventorySlot::EquipmentFeet => InventoryType::Feet,
            InventorySlot::EquipmentWrists => InventoryType::Wrists,
            InventorySlot::EquipmentHands => InventoryType::Hands,
            InventorySlot::EquipmentFinger1 | InventorySlot::EquipmentFinger2 => {
                InventoryType::Finger
            }
            InventorySlot::EquipmentTrinket1 | InventorySlot::EquipmentTrinket2 => {
                InventoryType::Trinket
            }
            InventorySlot::EquipmentBack => InventoryType::Cloak,
            InventorySlot::EquipmentMainHand => InventoryType::WeaponMainHand,
            InventorySlot::EquipmentOffHand => InventoryType::WeaponOffHand,
            InventorySlot::EquipmentRanged => InventoryType::Ranged,
            InventorySlot::EquipmentTabard => InventoryType::Tabard,
        }
    }

    pub fn fetch_characters(
        conn: &PooledConnection<SqliteConnectionManager>,
        account_id: u32,
        data_store: Arc<DataStore>,
    ) -> Vec<CharEnumData> {
        let mut stmt = conn.prepare_cached("SELECT guid, name, race, class, level, gender, skin, face, hairstyle, haircolor, facialstyle, map_id, zone_id, position_x, position_y, position_z FROM characters WHERE account_id = :account_id").unwrap();
        let chars = stmt
            .query_map(named_params! { ":account_id": account_id }, |row| {
                let char_guid: u64 = row.get("guid").unwrap();

                let mut stmt_gear = conn.prepare_cached("SELECT items.entry, character_inventory.slot \
                                                        FROM characters \
                                                        JOIN character_inventory ON character_inventory.character_guid = characters.guid \
                                                        JOIN items ON items.guid = character_inventory.item_guid \
                                                        WHERE characters.guid = :guid AND character_inventory.slot BETWEEN :start AND :end").unwrap();

                let equipment: HashMap<InventoryType, &ItemRecord> = stmt_gear.query_map(named_params! {
                    ":guid": char_guid,
                    ":start": InventorySlot::EquipmentHead as u32,
                    ":end": InventorySlot::EquipmentTabard as u32,
                }, |item_row| {
                    let item_entry: u32 = item_row.get(0).unwrap();
                    let inv_slot: InventorySlot = InventorySlot::n(item_row.get::<usize, u32>(1).unwrap()).unwrap();
                    let inv_type = Self::to_inventory_type_for_enum(&inv_slot);
                    let item_dbc = data_store.get_item_record(item_entry).expect("Unknown item found in CharacterRepository::fetch_characters");

                    Ok((inv_type, item_dbc))
                }).unwrap().into_iter().map(|res| res.unwrap()).collect();

                let equipment = vec![
                    InventoryType::Head,
                    InventoryType::Neck,
                    InventoryType::Shoulders,
                    InventoryType::Body,
                    InventoryType::Chest,
                    InventoryType::Waist,
                    InventoryType::Legs,
                    InventoryType::Feet,
                    InventoryType::Wrists,
                    InventoryType::Hands,
                    InventoryType::Finger,
                    InventoryType::Finger,
                    InventoryType::Trinket,
                    InventoryType::Trinket,
                    InventoryType::Cloak,
                    InventoryType::WeaponMainHand,
                    InventoryType::WeaponOffHand,
                    InventoryType::Ranged,
                    InventoryType::Tabard,
                    InventoryType::NonEquip,
                ]
                .into_iter()
                .map(|inv_type| equipment.get(&inv_type).map(|&item_record| {
                    CharEnumEquip {
                        display_id: item_record.display_id,
                        slot: item_record.inventory_type as u8,
                        enchant_id: 0, // Enchants not implemented yet
                    }
                }).unwrap_or(CharEnumEquip::none(inv_type)))
                .collect();

                Ok(CharEnumData {
                    guid: row.get("guid").unwrap(),
                    name: row.get::<&str, String>("name").unwrap().try_into().unwrap(),
                    race: row.get("race").unwrap(),
                    class: row.get("class").unwrap(),
                    gender: row.get("gender").unwrap(),
                    skin: row.get("skin").unwrap(),
                    face: row.get("face").unwrap(),
                    hairstyle: row.get("hairstyle").unwrap(),
                    haircolor: row.get("haircolor").unwrap(),
                    facialstyle: row.get("facialstyle").unwrap(),
                    level: row.get("level").unwrap(),
                    zone: row.get("zone_id").unwrap(),
                    map: row.get("map_id").unwrap(),
                    position_x: row.get("position_x").unwrap(),
                    position_y: row.get("position_y").unwrap(),
                    position_z: row.get("position_z").unwrap(),
                    guild_id: 0,
                    flags: 0,
                    first_login: true, // TODO: Set to false after first login
                    pet_display_id: 0,
                    pet_level: 0,
                    pet_family: 0,
                    equipment,
                })
            })
            .unwrap();

        chars
            .filter(|res| res.is_ok())
            .map(|res| res.unwrap())
            .into_iter()
            .collect()
    }

    pub fn delete_character(
        conn: &PooledConnection<SqliteConnectionManager>,
        source: CmsgCharDelete,
        account_id: u32,
    ) {
        let mut stmt_delete = conn
            .prepare_cached(
                "DELETE FROM characters WHERE guid = :guid AND account_id = :account_id",
            )
            .unwrap();
        stmt_delete
            .execute(named_params! {
                ":guid": source.guid,
                ":account_id": account_id,
            })
            .unwrap();
    }

    pub fn fetch_basic_character_data(
        conn: &PooledConnection<SqliteConnectionManager>,
        guid: u64,
    ) -> Option<CharacterRecord> {
        let mut stmt = conn
            .prepare_cached("SELECT account_id, race, class, level, gender, name, haircolor, hairstyle, face, skin, facialstyle, map_id, zone_id, position_x, position_y, position_z, orientation, current_health FROM characters WHERE guid = :guid")
            .unwrap();
        let mut rows = stmt
            .query(named_params! {
                ":guid": guid,
            })
            .unwrap();

        if let Some(row) = rows.next().unwrap() {
            Some(CharacterRecord {
                guid,
                account_id: row.get("account_id").unwrap(),
                race: row.get("race").unwrap(),
                class: row.get("class").unwrap(),
                level: row.get("level").unwrap(),
                gender: row.get("gender").unwrap(),
                name: row.get("name").unwrap(),
                visual_features: PlayerVisualFeatures {
                    haircolor: row.get("haircolor").unwrap(),
                    hairstyle: row.get("hairstyle").unwrap(),
                    face: row.get("face").unwrap(),
                    skin: row.get("skin").unwrap(),
                    facialstyle: row.get("facialstyle").unwrap(),
                },
                position: WorldPosition {
                    // FIXME: for instanced maps
                    map_key: MapKey::for_continent(row.get("map_id").unwrap()),
                    zone: row.get("zone_id").unwrap(),
                    x: row.get("position_x").unwrap(),
                    y: row.get("position_y").unwrap(),
                    z: row.get("position_z").unwrap(),
                    o: row.get("orientation").unwrap(),
                },
                current_health: row.get("current_health").unwrap(),
            })
        } else {
            None
        }
    }

    pub fn fetch_character_spells(
        conn: &PooledConnection<SqliteConnectionManager>,
        guid: u64,
    ) -> Vec<u32> {
        let mut stmt = conn
            .prepare_cached(
                "SELECT spell_id FROM character_spells WHERE character_guid = :character_guid",
            )
            .unwrap();
        let mut rows = stmt
            .query(named_params! { ":character_guid": guid })
            .unwrap();

        let mut spells = Vec::new();
        while let Some(row) = rows.next().unwrap() {
            spells.push(row.get("spell_id").unwrap());
        }

        spells
    }

    pub fn fetch_character_skills(
        conn: &PooledConnection<SqliteConnectionManager>,
        guid: u64,
    ) -> Vec<CharacterSkill> {
        let mut stmt = conn.prepare_cached("SELECT skill_id, value, max_value FROM character_skills WHERE character_guid = :character_guid").unwrap();
        let rows = stmt
            .query_map(named_params! { ":character_guid": guid }, |row| {
                Ok(CharacterSkill {
                    skill_id: row.get("skill_id").unwrap(),
                    value: row.get("value").unwrap(),
                    max_value: row.get("max_value").unwrap(),
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn fetch_action_buttons(
        conn: &PooledConnection<SqliteConnectionManager>,
        guid: u64,
    ) -> Vec<ActionButton> {
        let mut stmt = conn.prepare_cached("SELECT position, action_type, action_value FROM character_action_buttons WHERE character_guid = :character_guid").unwrap();
        let rows = stmt
            .query_map(named_params! { ":character_guid": guid }, |row| {
                Ok(ActionButton {
                    position: row.get("position").unwrap(),
                    action_type: row.get("action_type").unwrap(),
                    action_value: row.get("action_value").unwrap(),
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn fetch_faction_standings(
        conn: &PooledConnection<SqliteConnectionManager>,
        guid: u64,
    ) -> Vec<CharacterReputationDbRecord> {
        let mut stmt = conn.prepare_cached("SELECT character_guid, faction_id, standing, flags FROM character_reputations WHERE character_guid = :character_guid").unwrap();
        let rows = stmt
            .query_map(named_params! { ":character_guid": guid }, |row| {
                Ok(CharacterReputationDbRecord {
                    character_guid: guid,
                    faction_id: row.get("faction_id").unwrap(),
                    standing: row.get("standing").unwrap(),
                    flags: row.get("flags").unwrap(),
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn add_item_to_inventory(
        transaction: &Transaction,
        character_guid: u64,
        item_guid: u64,
        slot: u32,
    ) {
        let mut stmt = transaction.prepare_cached("INSERT INTO character_inventory(character_guid, item_guid, slot) VALUES (:character_guid, :item_guid, :slot)").unwrap();
        stmt.execute(named_params! {
            ":character_guid": character_guid,
            ":item_guid": item_guid,
            ":slot": slot,
        })
        .unwrap();
    }

    pub fn add_spell_offline(transaction: &Transaction, character_guid: u64, spell_id: u32) {
        let mut stmt = transaction.prepare_cached("INSERT INTO character_spells(character_guid, spell_id) VALUES (:character_guid, :spell_id)").unwrap();
        stmt.execute(named_params! {
            ":character_guid": character_guid,
            ":spell_id": spell_id,
        })
        .unwrap();
    }

    pub fn add_skill_offline(
        transaction: &Transaction,
        character_guid: u64,
        skill_id: u32,
        value: u32,
        max_value: u32,
    ) {
        let mut stmt = transaction.prepare_cached("INSERT INTO character_skills(character_guid, skill_id, value, max_value) VALUES (:character_guid, :skill_id, :value, :max_value)").unwrap();

        stmt.execute(named_params! {
            ":character_guid": character_guid,
            ":skill_id": skill_id,
            ":value": value,
            ":max_value": max_value,
        })
        .unwrap();
    }

    pub fn add_action(
        transaction: &Transaction,
        character_guid: u64,
        position: u32,
        action_type: ActionButtonType,
        action_value: u32,
    ) {
        let mut stmt = transaction.prepare_cached("INSERT INTO character_action_buttons(character_guid, position, action_type, action_value) VALUES (:character_guid, :position, :action_type, :action_value)").unwrap();

        stmt.execute(named_params! {
            ":character_guid": character_guid,
            ":position": position,
            ":action_type": action_type as u32,
            ":action_value": action_value,
        })
        .unwrap();
    }

    pub fn add_reputation_offline(
        transaction: &Transaction,
        character_guid: u64,
        faction_id: u32,
        standing: i32,
        flags: u32,
    ) {
        let mut stmt = transaction.prepare_cached("INSERT INTO character_reputations(character_guid, faction_id, standing, flags) VALUES (:character_guid, :faction_id, :standing, :flags)").unwrap();

        stmt.execute(named_params! {
            ":character_guid": character_guid,
            ":faction_id": faction_id,
            ":standing": standing,
            ":flags": flags,
        })
        .unwrap();
    }

    pub fn load_quest_statuses(
        conn: &PooledConnection<SqliteConnectionManager>,
        guid: u64,
    ) -> HashMap<u32, QuestLogContext> {
        let mut stmt = conn.prepare_cached("SELECT quest_id, status, entity_count1, entity_count2, entity_count3, entity_count4 FROM character_quests WHERE character_guid = :character_guid").unwrap();

        let result = stmt
            .query_map(named_params! { ":character_guid": guid }, |row| {
                let quest_id: u32 = row.get("quest_id").unwrap();
                let status: PlayerQuestStatus = row
                    .get::<&str, u32>("status")
                    .map(|st| PlayerQuestStatus::n(st).unwrap())
                    .unwrap();

                let entity_counts: [u32; MAX_QUEST_OBJECTIVES_COUNT] = [
                    row.get("entity_count1").unwrap(),
                    row.get("entity_count2").unwrap(),
                    row.get("entity_count3").unwrap(),
                    row.get("entity_count4").unwrap(),
                ];

                Ok((
                    quest_id,
                    QuestLogContext {
                        slot: None,
                        status,
                        entity_counts,
                    },
                ))
            })
            .unwrap();

        result.filter_map(|res| res.ok()).collect()
    }

    // TODO: Migrate save_position_to_db and save_quest_status_to_db here
    pub fn save_to_db(
        transaction: &Transaction,
        player: &Player,
        health: &Health,
    ) -> Result<(), Error> {
        let mut stmt = transaction
            .prepare_cached(
                "UPDATE characters SET current_health = :current_health WHERE guid = :guid",
            )
            .unwrap();

        stmt.execute(named_params! {
            ":current_health": health.current(),
            ":guid": player.guid().counter(),
        })?;

        Ok(())
    }
}

pub struct CharacterRecord {
    pub guid: u64,
    pub account_id: u32,
    pub race: CharacterRace,
    pub class: CharacterClass,
    pub level: u8,
    pub gender: u8,
    pub name: String,
    pub position: WorldPosition,
    pub visual_features: PlayerVisualFeatures,
    pub current_health: u32,
}

pub struct CharacterReputationDbRecord {
    pub character_guid: u64,
    pub faction_id: u32,
    pub standing: i32,
    pub flags: u32,
}
