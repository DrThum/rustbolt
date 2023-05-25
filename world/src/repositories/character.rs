use std::{collections::HashMap, sync::Arc};

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, Transaction};

use crate::{
    datastore::{
        data_types::{ItemRecord, PlayerCreatePosition},
        DataStore,
    },
    entities::{player::PlayerVisualFeatures, position::WorldPosition},
    protocol::packets::{CharEnumData, CharEnumEquip, CmsgCharCreate, CmsgCharDelete},
    shared::constants::{InventorySlot, InventoryType},
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
            InventorySlot::EquipmentMainhand => InventoryType::WeaponMainHand,
            InventorySlot::EquipmentOffhand => InventoryType::WeaponOffHand,
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
            .prepare_cached("SELECT account_id, race, class, level, gender, name, haircolor, hairstyle, face, skin, facialstyle, map_id, zone_id, position_x, position_y, position_z, orientation FROM characters WHERE guid = :guid")
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
                    map: row.get("map_id").unwrap(),
                    zone: row.get("zone_id").unwrap(),
                    x: row.get("position_x").unwrap(),
                    y: row.get("position_y").unwrap(),
                    z: row.get("position_z").unwrap(),
                    o: row.get("orientation").unwrap(),
                },
            })
        } else {
            None
        }
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
}

pub struct CharacterRecord {
    pub guid: u64,
    pub account_id: u32,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub gender: u8,
    pub name: String,
    pub position: WorldPosition,
    pub visual_features: PlayerVisualFeatures,
}
