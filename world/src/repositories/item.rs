use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, Transaction};

use crate::{
    datastore::data_types::ItemTemplate,
    entities::item::Item,
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
    pub fn create(transaction: &Transaction, entry: u32, stack_count: u32) -> u32 {
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

        transaction.last_insert_rowid() as u32
    }

    pub fn update(transaction: &Transaction, item: &Item) {
        let mut stmt = transaction
            .prepare_cached("UPDATE items SET stack_count = :stack_count WHERE guid = :guid")
            .unwrap();
        stmt.execute(named_params! {
            ":stack_count": item.stack_count(),
            ":guid": item.guid().counter(),
        })
        .unwrap();
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
                use ItemTemplateColumnIndex::*;

                let mut item_stats: Vec<ItemTemplateStat> =
                    Vec::with_capacity(MAX_ITEM_TEMPLATE_STATS as usize);
                for index in 0..MAX_ITEM_TEMPLATE_STATS {
                    let base_index = StatType1 as usize + (index * 2);
                    item_stats.push(ItemTemplateStat {
                        stat_type: row.get(base_index).unwrap(),
                        stat_value: row.get(base_index + 1).unwrap(),
                    });
                }

                let mut item_damages: Vec<ItemTemplateDamage> =
                    Vec::with_capacity(MAX_ITEM_TEMPLATE_DAMAGES as usize);
                for index in 0..MAX_ITEM_TEMPLATE_DAMAGES {
                    let base_index = DmgMin1 as usize + (index * 3);
                    item_damages.push(ItemTemplateDamage {
                        damage_min: row.get(base_index).unwrap(),
                        damage_max: row.get(base_index + 1).unwrap(),
                        damage_type: row.get(base_index + 2).unwrap(),
                    });
                }

                let mut item_spells: Vec<ItemTemplateSpell> =
                    Vec::with_capacity(MAX_ITEM_TEMPLATE_SPELLS as usize);
                for index in 0..MAX_ITEM_TEMPLATE_SPELLS {
                    let base_index = SpellId1 as usize + (index * 7);
                    item_spells.push(ItemTemplateSpell {
                        id: row.get(base_index).unwrap(),
                        trigger_id: row.get(base_index + 1).unwrap(),
                        charges: row.get(base_index + 2).unwrap(),
                        ppm_rate: row.get(base_index + 3).unwrap(),
                        cooldown: row.get(base_index + 4).unwrap(),
                        category: row.get(base_index + 5).unwrap(),
                        category_cooldown: row.get(base_index + 6).unwrap(),
                    });
                }

                let mut item_sockets: Vec<ItemTemplateSocket> =
                    Vec::with_capacity(MAX_ITEM_TEMPLATE_SOCKETS as usize);
                for index in 0..MAX_ITEM_TEMPLATE_SOCKETS {
                    let base_index = SocketColor1 as usize + (index * 2);
                    item_sockets.push(ItemTemplateSocket {
                        color: row.get(base_index).unwrap(),
                        content: row.get(base_index + 1).unwrap(),
                    });
                }

                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(ItemTemplate {
                    entry: row.get(Entry as usize).unwrap(),
                    class: row.get(Class as usize).unwrap(),
                    subclass: row.get(SubClass as usize).unwrap(),
                    unk0: row.get(Unk0 as usize).unwrap(),
                    name: row.get(Name as usize).unwrap(),
                    display_id: row.get(DisplayId as usize).unwrap(),
                    quality: row.get(Quality as usize).unwrap(),
                    flags: row.get(Flags as usize).unwrap(),
                    buy_count: row.get(BuyCount as usize).unwrap(),
                    buy_price: row.get(BuyPrice as usize).unwrap(),
                    sell_price: row.get(SellPrice as usize).unwrap(),
                    inventory_type: row.get(InventoryType as usize).unwrap(),
                    allowable_class: row.get(AllowableClass as usize).unwrap(),
                    allowable_race: row.get(AllowableClass as usize).unwrap(),
                    item_level: row.get(ItemLevel as usize).unwrap(),
                    required_level: row.get(RequiredLevel as usize).unwrap(),
                    required_skill: row.get(RequiredSkill as usize).unwrap(),
                    required_skill_rank: row.get(RequiredSkillRank as usize).unwrap(),
                    required_spell: row.get(RequiredSpell as usize).unwrap(),
                    required_honor_rank: row.get(RequiredHonorRank as usize).unwrap(),
                    required_city_rank: row.get(RequiredCityRank as usize).unwrap(),
                    required_reputation_faction: row
                        .get(RequiredReputationFaction as usize)
                        .unwrap(),
                    required_reputation_rank: row.get(RequiredReputationRank as usize).unwrap(),
                    max_count: row.get(MaxCount as usize).unwrap(),
                    max_stack_count: row.get(MaxStackCount as usize).unwrap(),
                    container_slots: row.get(ContainerSlots as usize).unwrap(),
                    stats: item_stats,
                    damages: item_damages,
                    armor: row.get(Armor as usize).unwrap(),
                    holy_res: row.get(HolyRes as usize).unwrap(),
                    fire_res: row.get(FireRes as usize).unwrap(),
                    nature_res: row.get(NatureRes as usize).unwrap(),
                    frost_res: row.get(FrostRes as usize).unwrap(),
                    shadow_res: row.get(ShadowRes as usize).unwrap(),
                    arcane_res: row.get(ArcaneRes as usize).unwrap(),
                    delay: row.get(Delay as usize).unwrap(),
                    ammo_type: row.get(AmmoType as usize).unwrap(),
                    ranged_mod_range: row.get(RangedModRange as usize).unwrap(),
                    spells: item_spells,
                    bonding: row.get(Bonding as usize).unwrap(),
                    description: row.get(Description as usize).unwrap(),
                    page_text: row.get(PageText as usize).unwrap(),
                    language_id: row.get(LanguageId as usize).unwrap(),
                    page_material: row.get(PageMaterial as usize).unwrap(),
                    start_quest: row.get(StartQuest as usize).unwrap(),
                    lock_id: row.get(LockId as usize).unwrap(),
                    material: row.get(Material as usize).unwrap(),
                    sheath: row.get(Sheath as usize).unwrap(),
                    random_property: row.get(RandomProperty as usize).unwrap(),
                    random_suffix: row.get(RandomSuffix as usize).unwrap(),
                    block: row.get(Block as usize).unwrap(),
                    itemset: row.get(ItemSet as usize).unwrap(),
                    max_durability: row.get(MaxDurability as usize).unwrap(),
                    area: row.get(Area as usize).unwrap(),
                    map: row.get(Map as usize).unwrap(),
                    bag_family: row.get(BagFamily as usize).unwrap(),
                    totem_category: row.get(TotemCategory as usize).unwrap(),
                    sockets: item_sockets,
                    socket_bonus: row.get(SocketBonus as usize).unwrap(),
                    gem_properties: row.get(GemProperties as usize).unwrap(),
                    required_disenchant_skill: row.get(RequiredDisenchantSkill as usize).unwrap(),
                    armor_damage_modifier: row.get(ArmorDamageModifier as usize).unwrap(),
                    disenchant_id: row.get(DisenchantId as usize).unwrap(),
                    food_type: row.get(FoodType as usize).unwrap(),
                    min_money_loot: row.get(MinMoneyLoot as usize).unwrap(),
                    max_money_loot: row.get(MaxMoneyLoot as usize).unwrap(),
                    duration: row.get(Duration as usize).unwrap(),
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

#[allow(dead_code)]
enum ItemTemplateColumnIndex {
    Entry,
    Class,
    SubClass,
    Unk0,
    Name,
    DisplayId,
    Quality,
    Flags,
    BuyCount,
    BuyPrice,
    SellPrice,
    InventoryType,
    AllowableClass,
    AllowableRace,
    ItemLevel,
    RequiredLevel,
    RequiredSkill,
    RequiredSkillRank,
    RequiredSpell,
    RequiredHonorRank,
    RequiredCityRank,
    RequiredReputationFaction,
    RequiredReputationRank,
    MaxCount,
    MaxStackCount,
    ContainerSlots,
    StatType1,
    StatValue1,
    StatType2,
    StatValue2,
    StatType3,
    StatValue3,
    StatType4,
    StatValue4,
    StatType5,
    StatValue5,
    StatType6,
    StatValue6,
    StatType7,
    StatValue7,
    StatType8,
    StatValue8,
    StatType9,
    StatValue9,
    StatType10,
    StatValue10,
    DmgMin1,
    DmgMax1,
    DmgType1,
    DmgMin2,
    DmgMax2,
    DmgType2,
    DmgMin3,
    DmgMax3,
    DmgType3,
    DmgMin4,
    DmgMax4,
    DmgType4,
    DmgMin5,
    DmgMax5,
    DmgType5,
    Armor,
    HolyRes,
    FireRes,
    NatureRes,
    FrostRes,
    ShadowRes,
    ArcaneRes,
    Delay,
    AmmoType,
    RangedModRange,
    SpellId1,
    SpellTrigger1,
    SpellCharges1,
    SpellPPMRate1,
    SpellCooldown1,
    SpellCategory1,
    SpellCategoryCooldown1,
    SpellId2,
    SpellTrigger2,
    Spellcharges2,
    SpellPPMRate2,
    SpellCooldown2,
    SpellCategory2,
    SpellCategoryCooldown2,
    SpellId3,
    SpellTrigger3,
    SpellCharges3,
    SpellPPMRate3,
    SpellCooldown3,
    SpellCategory3,
    SpellCategoryCooldown3,
    SpellId4,
    SpellTrigger4,
    SpellCharges4,
    SpellPPMRate4,
    SpellCooldown4,
    SpellCategory4,
    SpellCategoryCooldown4,
    SpellId5,
    SpellTrigger5,
    SpellCharges5,
    SpellPPMRate5,
    SpellCooldown5,
    SpellCategory5,
    SpellCategoryCooldown5,
    Bonding,
    Description,
    PageText,
    LanguageId,
    PageMaterial,
    StartQuest,
    LockId,
    Material,
    Sheath,
    RandomProperty,
    RandomSuffix,
    Block,
    ItemSet,
    MaxDurability,
    Area,
    Map,
    BagFamily,
    TotemCategory,
    SocketColor1,
    SocketContent1,
    SocketColor2,
    SocketContent2,
    SocketColor3,
    SocketContent3,
    SocketBonus,
    GemProperties,
    RequiredDisenchantSkill,
    ArmorDamageModifier,
    DisenchantId,
    FoodType,
    MinMoneyLoot,
    MaxMoneyLoot,
    Duration,
}
