use std::{collections::HashMap, sync::Arc, time::SystemTime};

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, Error, Transaction};

use crate::{
    datastore::{
        data_types::{ItemRecord, PlayerCreatePosition},
        DataStore,
    },
    ecs::components::{cooldowns::Cooldowns, powers::Powers},
    entities::{
        object_guid::ObjectGuid,
        player::{
            player_data::{ActionButton, BindPoint, CharacterSkill, QuestLogContext},
            Player, PlayerVisualFeatures,
        },
        position::WorldPosition,
    },
    game::map_manager::MapKey,
    protocol::packets::{CharEnumData, CharEnumEquip, CmsgCharCreate, CmsgCharDelete},
    shared::constants::{
        ActionButtonType, CharacterClass, CharacterRace, InventorySlot, InventoryType,
        PlayerQuestStatus, PowerType, MAX_QUEST_OBJECTIVES_COUNT,
    },
};

use super::item::ItemRepository;

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
        health: u32,
        mana: u32,
    ) -> u32 {
        let mut stmt_create = transaction.prepare_cached(
            "INSERT INTO characters
            (guid, account_id, name, race, class, gender, skin, face, hairstyle, haircolor, facialstyle,
            map_id, zone_id, position_x, position_y, position_z, orientation, current_health, current_mana,
            bindpoint_map_id, bindpoint_area_id, bindpoint_position_x, bindpoint_position_y, bindpoint_position_z,
            bindpoint_orientation)
            VALUES
            (NULL, :account_id, :name, :race, :class, :gender, :skin, :face, :hairstyle, :haircolor, :facialstyle,
            :map, :zone, :x, :y, :z, :o, :current_health, :current_mana, :bindpoint_map_id, :bindpoint_area_id,
            :bindpoint_position_x, :bindpoint_position_y, :bindpoint_position_z, :bindpoint_orientation)
            ").unwrap();
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
                ":current_health": health,
                ":current_mana": mana,
                ":bindpoint_map_id": create_position.map,
                ":bindpoint_area_id": create_position.zone,
                ":bindpoint_position_x": create_position.x,
                ":bindpoint_position_y": create_position.y,
                ":bindpoint_position_z": create_position.z,
                ":bindpoint_orientation": create_position.o,
            })
            .unwrap();

        transaction.last_insert_rowid() as u32
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

    pub fn fetch_spell_cooldowns(
        conn: &PooledConnection<SqliteConnectionManager>,
        guid: u64,
    ) -> HashMap<u32, (Option<u32>, u64)> {
        let mut stmt = conn.prepare_cached("SELECT spell_id, item_id, cooldown_end_timestamp FROM character_spell_cooldowns WHERE character_guid = :guid").unwrap();
        let rows = stmt
            .query_map(named_params! { ":guid": guid }, |row| {
                let spell_id: u32 = row.get("spell_id").unwrap();
                let item_id: Option<u32> = row.get("item_id").unwrap();
                let timestamp: u64 = row.get("cooldown_end_timestamp").unwrap();

                Ok((spell_id, (item_id, timestamp)))
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
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
                }).unwrap().map(|res| res.unwrap()).collect();

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
                    name: row.get::<&str, String>("name").unwrap().into(),
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

        chars.flatten().collect()
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
            .prepare_cached(
                "SELECT account_id, race, class, level, gender, name, haircolor, hairstyle, face, skin, facialstyle,
                map_id, zone_id, position_x, position_y, position_z, orientation, current_health, current_mana, current_rage,
                current_energy, experience, money, bindpoint_map_id, bindpoint_area_id, bindpoint_position_x, bindpoint_position_y,
                bindpoint_position_z, bindpoint_orientation
                FROM characters WHERE guid = :guid")
            .unwrap();
        let mut rows = stmt
            .query(named_params! {
                ":guid": guid,
            })
            .unwrap();

        rows.next().unwrap().map(|row| CharacterRecord {
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
            current_mana: row.get("current_mana").unwrap(),
            current_rage: row.get("current_rage").unwrap(),
            current_energy: row.get("current_energy").unwrap(),
            experience: row.get("experience").unwrap(),
            money: row.get("money").unwrap(),
            bindpoint_map_id: row.get("bindpoint_map_id").unwrap(),
            bindpoint_area_id: row.get("bindpoint_area_id").unwrap(),
            bindpoint_position_x: row.get("bindpoint_position_x").unwrap(),
            bindpoint_position_y: row.get("bindpoint_position_y").unwrap(),
            bindpoint_position_z: row.get("bindpoint_position_z").unwrap(),
            bindpoint_orientation: row.get("bindpoint_orientation").unwrap(),
        })
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

    pub fn fetch_guid_and_position_by_name(
        conn: &PooledConnection<SqliteConnectionManager>,
        name: &str,
    ) -> Option<(ObjectGuid, WorldPosition)> {
        let mut stmt = conn.prepare_cached("SELECT guid, map_id, zone_id, position_x, position_y, position_z, orientation FROM characters WHERE name = :name").unwrap();
        let mut rows = stmt.query(named_params! { ":name": name }).unwrap();

        rows.next().unwrap().and_then(|row| {
            let guid = ObjectGuid::from_raw(row.get("guid").unwrap());

            let position = WorldPosition {
                map_key: MapKey::for_continent(row.get("map_id").unwrap()),
                zone: row.get("zone_id").unwrap(),
                x: row.get("position_x").unwrap(),
                y: row.get("position_y").unwrap(),
                z: row.get("position_z").unwrap(),
                o: row.get("orientation").unwrap(),
            };

            guid.map(|guid| (guid, position))
        })
    }

    pub fn add_item_to_inventory(
        transaction: &Transaction,
        character_guid: u32,
        item_guid: u32,
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

    pub fn add_spell_offline(transaction: &Transaction, character_guid: u32, spell_id: u32) {
        let mut stmt = transaction.prepare_cached("INSERT INTO character_spells(character_guid, spell_id) VALUES (:character_guid, :spell_id)").unwrap();
        stmt.execute(named_params! {
            ":character_guid": character_guid,
            ":spell_id": spell_id,
        })
        .unwrap();
    }

    pub fn add_skill_offline(
        transaction: &Transaction,
        character_guid: u32,
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
        character_guid: u32,
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
        character_guid: u32,
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

    pub fn save_to_db(
        transaction: &Transaction,
        player: &mut Player,
        powers: &Powers,
        position: &WorldPosition,
        cooldowns: &Cooldowns,
    ) -> Result<(), Error> {
        let guid = player.guid().counter();

        // Save character data
        let mut stmt = transaction
            .prepare_cached(
                "UPDATE characters SET
                level = :level,
                map_id = :map_id, zone_id = :zone_id, position_x = :x, position_y = :y, position_z = :z, orientation = :o,
                current_health = :current_health, current_mana = :current_mana, current_rage = :current_rage, current_energy = :current_energy,
                experience = :experience, money = :money, bindpoint_map_id = :bindpoint_map_id, bindpoint_area_id = :bindpoint_area_id,
                bindpoint_position_x = :bindpoint_position_x, bindpoint_position_y = :bindpoint_position_y, bindpoint_position_z = :bindpoint_position_z,
                bindpoint_orientation = :bindpoint_orientation
                WHERE guid = :guid",
            )
            .unwrap();

        let bindpoint = player.bindpoint();

        stmt.execute(named_params! {
            ":level": player.level(),
            ":map_id": position.map_key.map_id,
            ":zone_id": position.zone,
            ":x": position.x,
            ":y": position.y,
            ":z": position.z,
            ":o": position.o,
            ":current_health": powers.current_health(),
            ":current_mana": powers.current_power(&PowerType::Mana),
            ":current_rage": powers.current_power(&PowerType::Rage),
            ":current_energy": powers.current_power(&PowerType::Energy),
            ":experience": player.experience(),
            ":money": player.money(),
            ":guid": guid,
            ":bindpoint_map_id": bindpoint.map_id,
            ":bindpoint_area_id": bindpoint.area_id,
            ":bindpoint_position_x": bindpoint.x,
            ":bindpoint_position_y": bindpoint.y,
            ":bindpoint_position_z": bindpoint.z,
            ":bindpoint_orientation": bindpoint.o,
        })?;

        // Save quest data
        let mut stmt = transaction
            .prepare_cached("DELETE FROM character_quests WHERE character_guid = :guid")
            .unwrap();
        stmt.execute(named_params! { ":guid": guid })?;

        // TODO: Save the current timer here
        let mut stmt = transaction.prepare_cached("INSERT INTO character_quests (character_guid, quest_id, status, entity_count1, entity_count2, entity_count3, entity_count4) VALUES (:guid, :quest_id, :status, :entity_count1, :entity_count2, :entity_count3, :entity_count4)").unwrap();
        player
            .quest_statuses()
            .iter()
            .for_each(|(quest_id, context)| {
                stmt.execute(named_params! {
                    ":guid": guid,
                    ":quest_id": quest_id,
                    ":status": context.status as u32,
                    ":entity_count1": context.entity_counts[0],
                    ":entity_count2": context.entity_counts[1],
                    ":entity_count3": context.entity_counts[2],
                    ":entity_count4": context.entity_counts[3],
                })
                .unwrap();
            });

        // Save inventory data
        // FIXME: We're creating orphaned items in table `items` with the current implementation
        // because we don't DELETE FROM items for deleted items
        let mut stmt = transaction
            .prepare_cached("DELETE FROM character_inventory WHERE character_guid = :guid")
            .unwrap();
        stmt.execute(named_params! {":guid": guid})?;

        for (slot, item) in player.inventory().list() {
            if item.needs_db_save() {
                ItemRepository::upsert(transaction, item);
            }

            let mut stmt = transaction.prepare_cached("INSERT INTO character_inventory(character_guid, item_guid, slot) VALUES (:character_guid, :item_guid, :slot)").unwrap();
            stmt.execute(named_params! {
                ":character_guid": guid,
                ":item_guid": item.guid().counter(),
                ":slot": slot,
            })
            .unwrap();
        }

        player.inventory_mut().mark_saved();

        // Save spell cooldowns
        let mut stmt = transaction
            .prepare_cached("DELETE FROM character_spell_cooldowns WHERE character_guid = :guid")
            .unwrap();
        stmt.execute(named_params! {":guid": guid})?;

        let now = SystemTime::now();
        for (spell_id, cooldown) in cooldowns.list() {
            if cooldown.end < now {
                continue; // Skip expired cooldowns
            }

            let mut stmt = transaction.prepare_cached("INSERT INTO character_spell_cooldowns(character_guid, spell_id, item_id, cooldown_end_timestamp) VALUES (:guid, :spell_id, :item_id, :timestamp)").unwrap();
            stmt.execute(named_params! {
                ":guid": guid,
                ":spell_id": spell_id,
                ":item_id": cooldown.item_id,
                ":timestamp": cooldown.end.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64,
            })?;
        }

        // Save action buttons
        let mut stmt = transaction
            .prepare_cached("DELETE FROM character_action_buttons WHERE character_guid = :guid")
            .unwrap();
        stmt.execute(named_params! {":guid": guid})?;
        for (_, action) in player.action_buttons() {
            Self::add_action(
                transaction,
                guid,
                action.position,
                action.action_type,
                action.action_value,
            );
        }

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
    pub current_mana: u32,
    pub current_rage: u32,
    pub current_energy: u32,
    pub experience: u32,
    pub money: u32,
    pub bindpoint_map_id: u32,
    pub bindpoint_area_id: u32,
    pub bindpoint_position_x: f32,
    pub bindpoint_position_y: f32,
    pub bindpoint_position_z: f32,
    pub bindpoint_orientation: f32,
}

impl CharacterRecord {
    pub fn bindpoint(&self) -> BindPoint {
        BindPoint {
            map_id: self.bindpoint_map_id,
            area_id: self.bindpoint_area_id,
            x: self.bindpoint_position_x,
            y: self.bindpoint_position_y,
            z: self.bindpoint_position_z,
            o: self.bindpoint_orientation,
        }
    }
}

pub struct CharacterReputationDbRecord {
    pub character_guid: u64,
    pub faction_id: u32,
    pub standing: i32,
    pub flags: u32,
}
