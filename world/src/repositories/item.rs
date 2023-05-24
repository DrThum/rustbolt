use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, Transaction};

use crate::{
    datastore::data_types::ItemTemplate,
    protocol::packets::{
        ItemTemplateDamage, ItemTemplateSocket, ItemTemplateSpell, ItemTemplateStat,
    },
    shared::constants::{
        MAX_ITEM_TEMPLATE_DAMAGES, MAX_ITEM_TEMPLATE_SOCKETS, MAX_ITEM_TEMPLATE_SPELLS,
        MAX_ITEM_TEMPLATE_STATS,
    },
};

pub struct ItemRepository;

impl ItemRepository {
    pub fn create(transaction: &Transaction, entry: u32, stack_count: u32) -> u64 {
        assert!(
            stack_count > 0,
            "Cannot create an item in DB with stack_count = 0"
        );

        let mut stmt = transaction
            .prepare_cached(
                "INSERT INTO items(guid, entry, stack_count) VALUES (NULL, :entry, :stack_count)",
            )
            .unwrap();
        stmt.execute(named_params! {
            ":entry": entry,
            ":stack_count": stack_count,
        })
        .unwrap();

        transaction.last_insert_rowid() as u64
    }

    pub fn load_player_inventory(
        conn: &PooledConnection<SqliteConnectionManager>,
        player_guid: u32,
    ) -> Vec<ItemDbRecord> {
        let mut stmt = conn.prepare_cached("SELECT items.guid AS item_guid, items.entry AS item_entry, items.stack_count AS item_stack_count, character_inventory.character_guid AS character_guid, character_inventory.slot AS slot FROM items JOIN character_inventory ON character_inventory.item_guid = items.guid WHERE character_inventory.character_guid = :player_guid").unwrap();

        let result = stmt
            .query_map(named_params! { ":player_guid": player_guid }, |row| {
                let guid: u32 = row.get("item_guid").unwrap();
                let item_entry: u32 = row.get("item_entry").unwrap();
                let stack_count: u32 = row.get("item_stack_count").unwrap();
                let owner_guid: u64 = row.get("character_guid").unwrap();
                let slot: u32 = row.get("slot").unwrap();

                Ok(ItemDbRecord {
                    guid,
                    entry: item_entry,
                    stack_count,
                    owner_guid: Some(owner_guid),
                    slot,
                })
            })
            .unwrap();

        result
            .filter(|res| res.is_ok())
            .map(|res| res.unwrap())
            .into_iter()
            .collect()
    }

    pub fn load_templates(conn: &PooledConnection<SqliteConnectionManager>) -> Vec<ItemTemplate> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(entry) FROM item_templates")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare_cached("SELECT entry, class, subclass, unk0, name, display_id, quality, flags, buy_count, buy_price, sell_price, inventory_type, allowable_class, allowable_race, item_level, required_level, required_skill, required_skill_rank, required_spell, required_honor_rank, required_city_rank, required_reputation_faction, required_reputation_rank, max_count, max_stack_count, container_slots, stat_type1, stat_value1, stat_type2, stat_value2, stat_type3, stat_value3, stat_type4, stat_value4, stat_type5, stat_value5, stat_type6, stat_value6, stat_type7, stat_value7, stat_type8, stat_value8, stat_type9, stat_value9, stat_type10, stat_value10, dmg_min1, dmg_max1, dmg_type1, dmg_min2, dmg_max2, dmg_type2, dmg_min3, dmg_max3, dmg_type3, dmg_min4, dmg_max4, dmg_type4, dmg_min5, dmg_max5, dmg_type5, armor, holy_res, fire_res, nature_res, frost_res, shadow_res, arcane_res, delay, ammo_type, ranged_mod_range, spellid_1, spelltrigger_1, spellcharges_1, spellppm_rate_1, spellcooldown_1, spellcategory_1, spellcategorycooldown_1, spellid_2, spelltrigger_2, spellcharges_2, spellppm_rate_2, spellcooldown_2, spellcategory_2, spellcategorycooldown_2, spellid_3, spelltrigger_3, spellcharges_3, spellppm_rate_3, spellcooldown_3, spellcategory_3, spellcategorycooldown_3, spellid_4, spelltrigger_4, spellcharges_4, spellppm_rate_4, spellcooldown_4, spellcategory_4, spellcategorycooldown_4, spellid_5, spelltrigger_5, spellcharges_5, spellppm_rate_5, spellcooldown_5, spellcategory_5, spellcategorycooldown_5, bonding, description, page_text, language_id, page_material, start_quest, lock_id, material, sheath, random_property, random_suffix, block, itemset, max_durability, area, map, bag_family, totem_category, socket_color_1, socket_content_1, socket_color_2, socket_content_2, socket_color_3, socket_content_3, socket_bonus, gem_properties, required_disenchant_skill, armor_damage_modifier, disenchant_id, food_type, min_money_loot, max_money_loot, duration FROM item_templates ORDER BY entry").unwrap();

        let result = stmt
            .query_map([], |row| {
                let mut item_stats: Vec<ItemTemplateStat> =
                    Vec::with_capacity(MAX_ITEM_TEMPLATE_STATS as usize);
                for index in 1..=MAX_ITEM_TEMPLATE_STATS {
                    item_stats.push(ItemTemplateStat {
                        stat_type: row.get(format!("stat_type{}", index).as_str()).unwrap(),
                        stat_value: row.get(format!("stat_value{}", index).as_str()).unwrap(),
                    });
                }

                let mut item_damages: Vec<ItemTemplateDamage> =
                    Vec::with_capacity(MAX_ITEM_TEMPLATE_DAMAGES as usize);
                for index in 1..=MAX_ITEM_TEMPLATE_DAMAGES {
                    item_damages.push(ItemTemplateDamage {
                        damage_min: row.get(format!("dmg_min{}", index).as_str()).unwrap(),
                        damage_max: row.get(format!("dmg_max{}", index).as_str()).unwrap(),
                        damage_type: row.get(format!("dmg_type{}", index).as_str()).unwrap(),
                    });
                }

                let mut item_spells: Vec<ItemTemplateSpell> =
                    Vec::with_capacity(MAX_ITEM_TEMPLATE_SPELLS as usize);
                for index in 1..=MAX_ITEM_TEMPLATE_SPELLS {
                    item_spells.push(ItemTemplateSpell {
                        id: row.get(format!("spellid_{}", index).as_str()).unwrap(),
                        trigger_id: row.get(format!("spelltrigger_{}", index).as_str()).unwrap(),
                        charges: row.get(format!("spellcharges_{}", index).as_str()).unwrap(),
                        ppm_rate: row
                            .get(format!("spellppm_rate_{}", index).as_str())
                            .unwrap(),
                        cooldown: row
                            .get(format!("spellcooldown_{}", index).as_str())
                            .unwrap(),
                        category: row
                            .get(format!("spellcategory_{}", index).as_str())
                            .unwrap(),
                        category_cooldown: row
                            .get(format!("spellcategorycooldown_{}", index).as_str())
                            .unwrap(),
                    });
                }

                let mut item_sockets: Vec<ItemTemplateSocket> =
                    Vec::with_capacity(MAX_ITEM_TEMPLATE_SOCKETS as usize);
                for index in 1..=MAX_ITEM_TEMPLATE_SOCKETS {
                    item_sockets.push(ItemTemplateSocket {
                        color: row.get(format!("socket_color_{}", index).as_str()).unwrap(),
                        content: row
                            .get(format!("socket_content_{}", index).as_str())
                            .unwrap(),
                    });
                }

                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(ItemTemplate {
                    entry: row.get("entry").unwrap(),
                    class: row.get("class").unwrap(),
                    subclass: row.get("subclass").unwrap(),
                    unk0: row.get("unk0").unwrap(),
                    name: row.get("name").unwrap(),
                    display_id: row.get("display_id").unwrap(),
                    quality: row.get("quality").unwrap(),
                    flags: row.get("flags").unwrap(),
                    buy_count: row.get("buy_count").unwrap(),
                    buy_price: row.get("buy_price").unwrap(),
                    sell_price: row.get("sell_price").unwrap(),
                    inventory_type: row.get("inventory_type").unwrap(),
                    allowable_class: row.get("allowable_class").unwrap(),
                    allowable_race: row.get("allowable_race").unwrap(),
                    item_level: row.get("item_level").unwrap(),
                    required_level: row.get("required_level").unwrap(),
                    required_skill: row.get("required_skill").unwrap(),
                    required_skill_rank: row.get("required_skill_rank").unwrap(),
                    required_spell: row.get("required_spell").unwrap(),
                    required_honor_rank: row.get("required_honor_rank").unwrap(),
                    required_city_rank: row.get("required_city_rank").unwrap(),
                    required_reputation_faction: row.get("required_reputation_faction").unwrap(),
                    required_reputation_rank: row.get("required_reputation_rank").unwrap(),
                    max_count: row.get("max_count").unwrap(),
                    max_stack_count: row.get("max_stack_count").unwrap(),
                    container_slots: row.get("container_slots").unwrap(),
                    stats: item_stats,
                    damages: item_damages,
                    armor: row.get("armor").unwrap(),
                    holy_res: row.get("holy_res").unwrap(),
                    fire_res: row.get("fire_res").unwrap(),
                    nature_res: row.get("nature_res").unwrap(),
                    frost_res: row.get("frost_res").unwrap(),
                    shadow_res: row.get("shadow_res").unwrap(),
                    arcane_res: row.get("arcane_res").unwrap(),
                    delay: row.get("delay").unwrap(),
                    ammo_type: row.get("ammo_type").unwrap(),
                    ranged_mod_range: row.get("ranged_mod_range").unwrap(),
                    spells: item_spells,
                    bonding: row.get("bonding").unwrap(),
                    description: row.get("description").unwrap(),
                    page_text: row.get("page_text").unwrap(),
                    language_id: row.get("language_id").unwrap(),
                    page_material: row.get("page_material").unwrap(),
                    start_quest: row.get("start_quest").unwrap(),
                    lock_id: row.get("lock_id").unwrap(),
                    material: row.get("material").unwrap(),
                    sheath: row.get("sheath").unwrap(),
                    random_property: row.get("random_property").unwrap(),
                    random_suffix: row.get("random_suffix").unwrap(),
                    block: row.get("block").unwrap(),
                    itemset: row.get("itemset").unwrap(),
                    max_durability: row.get("max_durability").unwrap(),
                    area: row.get("area").unwrap(),
                    map: row.get("map").unwrap(),
                    bag_family: row.get("bag_family").unwrap(),
                    totem_category: row.get("totem_category").unwrap(),
                    sockets: item_sockets,
                    socket_bonus: row.get("socket_bonus").unwrap(),
                    gem_properties: row.get("gem_properties").unwrap(),
                    required_disenchant_skill: row.get("required_disenchant_skill").unwrap(),
                    armor_damage_modifier: row.get("armor_damage_modifier").unwrap(),
                    disenchant_id: row.get("disenchant_id").unwrap(),
                    food_type: row.get("food_type").unwrap(),
                    min_money_loot: row.get("min_money_loot").unwrap(),
                    max_money_loot: row.get("max_money_loot").unwrap(),
                    duration: row.get("duration").unwrap(),
                })
            })
            .unwrap();

        result
            .filter(|res| res.is_ok())
            .map(|res| res.unwrap())
            .into_iter()
            .collect()
    }
}

pub struct ItemDbRecord {
    pub guid: u32,
    pub entry: u32,
    pub stack_count: u32,
    pub owner_guid: Option<u64>,
    pub slot: u32,
}
