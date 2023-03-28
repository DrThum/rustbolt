use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::named_params;

use crate::{
    entities::player::PlayerVisualFeatures,
    protocol::packets::{CharEnumData, CharEnumEquip, CmsgCharCreate, CmsgCharDelete},
    shared::constants::InventoryType,
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
        conn: &PooledConnection<SqliteConnectionManager>,
        source: CmsgCharCreate,
        account_id: u32,
    ) {
        let mut stmt_create = conn.prepare_cached("INSERT INTO characters (guid, account_id, name, race, class, gender, skin, face, hairstyle, haircolor, facialstyle) VALUES (NULL, :account_id, :name, :race, :class, :gender, :skin, :face, :hairstyle, :haircolor, :facialstyle)").unwrap();
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
            })
            .unwrap();
    }

    pub fn fetch_characters(
        conn: PooledConnection<SqliteConnectionManager>,
        account_id: u32,
    ) -> Vec<CharEnumData> {
        let mut stmt = conn.prepare_cached("SELECT guid, name, race, class, level, gender, skin, face, hairstyle, haircolor, facialstyle FROM characters WHERE account_id = :account_id").unwrap();
        let chars = stmt
            .query_map(named_params! { ":account_id": account_id }, |row| {
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
                .map(|inv_type| CharEnumEquip::none(inv_type))
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
                    area: 85,
                    map: 0,
                    position_x: 0.0,
                    position_y: 0.0,
                    position_z: 0.0,
                    guild_id: 0,
                    flags: 0,
                    first_login: 1, // FIXME: bool
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
        conn: PooledConnection<SqliteConnectionManager>,
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
        conn: &mut PooledConnection<SqliteConnectionManager>,
        guid: u64,
        account_id: u32,
    ) -> Option<CharacterRecord> {
        let mut stmt = conn
            .prepare("SELECT race, class, level, gender, haircolor, hairstyle, face, skin, facialstyle FROM characters WHERE guid = :guid AND account_id = :account_id")
            .unwrap();
        let mut rows = stmt
            .query(named_params! {
                ":guid": guid,
                ":account_id": account_id,
            })
            .unwrap();

        if let Some(row) = rows.next().unwrap() {
            Some(CharacterRecord {
                race: row.get("race").unwrap(),
                class: row.get("class").unwrap(),
                level: row.get("level").unwrap(),
                gender: row.get("gender").unwrap(),
                visual_features: PlayerVisualFeatures {
                    haircolor: row.get("haircolor").unwrap(),
                    hairstyle: row.get("hairstyle").unwrap(),
                    face: row.get("face").unwrap(),
                    skin: row.get("skin").unwrap(),
                    facialstyle: row.get("facialstyle").unwrap(),
                },
            })
        } else {
            None
        }
    }
}

pub struct CharacterRecord {
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub gender: u8,
    pub visual_features: PlayerVisualFeatures,
}
