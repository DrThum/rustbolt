use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use enumflags2::make_bitflags;
use log::{error, warn};
use parking_lot::RwLock;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Error;
use shared::utils::value_range::ValueRange;
use shipyard::{Component, EntityId};
use strum::IntoEnumIterator;

use crate::{
    datastore::{
        data_types::{PlayerCreatePosition, QuestTemplate, GAME_TABLE_MAX_LEVEL},
        DataStore,
    },
    entities::player::player_data::FactionStanding,
    game::world_context::WorldContext,
    protocol::{
        packets::{CmsgCharCreate, SmsgCreateObject, SmsgItemPushResult, SmsgQuestUpdateAddKill},
        server::ServerMessage,
    },
    repositories::{character::CharacterRepository, item::ItemRepository},
    session::world_session::WorldSession,
    shared::constants::{
        AbilityLearnType, CharacterClass, CharacterClassBit, CharacterRaceBit, Gender,
        HighGuidType, InventoryResult, InventorySlot, InventoryType, ItemClass,
        ItemSubclassConsumable, ObjectTypeId, ObjectTypeMask, PlayerQuestStatus, PowerType,
        QuestSlotState, QuestStartError, SkillRangeType, SpellSchool, UnitAttribute, UnitFlags,
        WeaponAttackType, BASE_ATTACK_TIME, BASE_DAMAGE, MAX_QUESTS_IN_LOG,
        MAX_QUEST_OBJECTIVES_COUNT, PLAYER_DEFAULT_BOUNDING_RADIUS, PLAYER_DEFAULT_COMBAT_REACH,
    },
};

use self::{
    player_data::{ActionButton, QuestLogContext},
    player_inventory::PlayerInventory,
};

use super::{
    internal_values::{InternalValues, QuestSlotOffset, QUEST_SLOT_OFFSETS_COUNT},
    item::Item,
    object_guid::ObjectGuid,
    update::{
        CreateData, MovementUpdateData, UpdateBlockBuilder, UpdateData, UpdateFlag, UpdateType,
    },
    update_fields::*,
};

pub mod experience;
pub mod player_data;
pub mod player_inventory;

#[derive(Component)]
pub struct Player {
    pub session: Arc<WorldSession>,
    world_context: Arc<WorldContext>,
    guid: ObjectGuid,
    pub name: String,
    pub internal_values: Arc<RwLock<InternalValues>>,
    inventory: PlayerInventory,
    spells: Vec<u32>,
    action_buttons: HashMap<usize, ActionButton>,
    faction_standings: HashMap<u32, FactionStanding>,
    quest_statuses: HashMap<u32, QuestLogContext>,
    in_combat_with: RwLock<HashSet<ObjectGuid>>,
    currently_looting: Option<EntityId>,
    partial_regen_period_end: Instant, // "Five Seconds Rule", partial mana regen before, full regen after
}

impl Player {
    pub fn create_in_db(
        conn: &mut PooledConnection<SqliteConnectionManager>,
        creation_payload: &CmsgCharCreate,
        account_id: u32,
        world_context: Arc<WorldContext>,
    ) -> Result<(), Error> {
        let data_store = world_context.data_store.clone();
        let transaction = conn.transaction()?;

        let create_position: &PlayerCreatePosition = data_store
            .get_player_create_position(creation_payload.race as u32, creation_payload.class as u32)
            .expect("missing player create position in DB");

        let base_health_mana_record = data_store
            .get_player_base_health_mana(CharacterClass::n(creation_payload.class).unwrap(), 1)
            .expect("unable to retrieve base health/mana for this class/level combination");

        let character_guid = CharacterRepository::create_character(
            &transaction,
            creation_payload,
            account_id,
            create_position,
            base_health_mana_record.base_health,
        );

        let start_items = data_store
            .get_char_start_outfit(
                creation_payload.race,
                creation_payload.class,
                creation_payload.gender,
            )
            .map(|outfit| &outfit.items)
            .expect("Attempt to create a character with no corresponding CharStartOutfit");

        let mut current_bag_slot: u32 = InventorySlot::BACKPACK_START;
        for start_item in start_items {
            if let Some(template) = data_store.get_item_template(start_item.id) {
                let stack_count = match (
                    ItemClass::n(template.class),
                    ItemSubclassConsumable::n(template.subclass),
                ) {
                    (Some(ItemClass::Consumable), Some(ItemSubclassConsumable::Food)) => {
                        match template.spells[0].category {
                            11 => 4, // Food
                            59 => 2, // Drink
                            _ => template.buy_count,
                        }
                    }
                    _ => template.buy_count,
                };

                // Note: this won't work for multiple rings or trinkets but it shouldn't happen with
                // starting gear
                let slot = match start_item.inventory_type {
                    InventoryType::NonEquip
                    | InventoryType::Ammo
                    | InventoryType::Thrown
                    | InventoryType::Bag => {
                        let res = current_bag_slot;
                        current_bag_slot += 1;
                        res
                    }
                    InventoryType::Head => InventorySlot::EquipmentHead as u32,
                    InventoryType::Neck => InventorySlot::EquipmentNeck as u32,
                    InventoryType::Shoulders => InventorySlot::EquipmentShoulders as u32,
                    InventoryType::Body => InventorySlot::EquipmentBody as u32,
                    InventoryType::Chest | InventoryType::Robe => {
                        InventorySlot::EquipmentChest as u32
                    }
                    InventoryType::Waist => InventorySlot::EquipmentWaist as u32,
                    InventoryType::Legs => InventorySlot::EquipmentLegs as u32,
                    InventoryType::Feet => InventorySlot::EquipmentFeet as u32,
                    InventoryType::Wrists => InventorySlot::EquipmentWrists as u32,
                    InventoryType::Hands => InventorySlot::EquipmentHands as u32,
                    InventoryType::Finger => InventorySlot::EquipmentFinger1 as u32,
                    InventoryType::Trinket => InventorySlot::EquipmentTrinket1 as u32,
                    InventoryType::Weapon
                    | InventoryType::Holdable
                    | InventoryType::TwoHandWeapon
                    | InventoryType::WeaponMainHand => InventorySlot::EquipmentMainHand as u32,
                    InventoryType::Shield
                    | InventoryType::WeaponOffHand
                    | InventoryType::Quiver => InventorySlot::EquipmentOffHand as u32,
                    InventoryType::Ranged | InventoryType::RangedRight | InventoryType::Relic => {
                        InventorySlot::EquipmentRanged as u32
                    }
                    InventoryType::Cloak => InventorySlot::EquipmentBack as u32,

                    InventoryType::Tabard => InventorySlot::EquipmentTabard as u32,
                };

                let item_guid = world_context.next_item_guid();
                ItemRepository::create(&transaction, item_guid, start_item.id, stack_count);
                CharacterRepository::add_item_to_inventory(
                    &transaction,
                    character_guid,
                    item_guid,
                    slot,
                );
            } else {
                error!("Unknown item {} in CharStartOutfit", start_item.id);
            }
        }

        let start_spells = data_store
            .get_player_create_spells(creation_payload.race as u32, creation_payload.class as u32)
            .expect("Missing player create spells in DB");

        let mut added_skill_ids: HashSet<u32> = HashSet::new();
        for spell_id in start_spells {
            if let Some(spell_record) = data_store.get_spell_record(*spell_id) {
                CharacterRepository::add_spell_offline(&transaction, character_guid, *spell_id);

                if let Some(learnable_skill) = spell_record.learnable_skill() {
                    if !added_skill_ids.contains(&learnable_skill.skill_id) {
                        CharacterRepository::add_skill_offline(
                            &transaction,
                            character_guid,
                            learnable_skill.skill_id,
                            learnable_skill.value,
                            learnable_skill.max_value,
                        );

                        added_skill_ids.insert(learnable_skill.skill_id);
                    }
                } else if let Some(skill_abilities) =
                    data_store.get_skill_line_ability_by_spell(*spell_id)
                {
                    for skill_ability in skill_abilities {
                        if let Some(skill_line) =
                            data_store.get_skill_line_record(skill_ability.skill_id as u32)
                        {
                            if skill_ability.learn_on_get_skill
                                == AbilityLearnType::LearnedOnGetRaceOrClassSkill
                            {
                                let (value, max_value) = match skill_line.range_type() {
                                    SkillRangeType::Language => (300, 300),
                                    SkillRangeType::Level => (1, 1 /* level */ * 5),
                                    SkillRangeType::Mono => (1, 1),
                                    _ => (0, 0),
                                };

                                if value != 0
                                    && max_value != 0
                                    && !added_skill_ids.contains(&(skill_ability.skill_id as u32))
                                {
                                    CharacterRepository::add_skill_offline(
                                        &transaction,
                                        character_guid,
                                        skill_ability.skill_id as u32,
                                        value,
                                        max_value,
                                    );

                                    added_skill_ids.insert(skill_ability.skill_id as u32);
                                }
                            }
                        }
                    }
                }
            }
        }

        let start_actions = data_store
            .get_player_create_action_buttons(
                creation_payload.race as u32,
                creation_payload.class as u32,
            )
            .expect("missing player create action buttons in DB");

        for action_button in start_actions {
            CharacterRepository::add_action(
                &transaction,
                character_guid,
                action_button.position,
                action_button.action_type,
                action_button.action_value,
            );
        }

        let start_reputations = data_store.get_starting_factions(
            CharacterRaceBit::n(1 << (creation_payload.race - 1)).unwrap(),
            CharacterClassBit::n(1 << (creation_payload.class - 1)).unwrap(),
        );

        for reputation in start_reputations {
            CharacterRepository::add_reputation_offline(
                &transaction,
                character_guid,
                reputation.0,
                0,
                reputation.1,
            );
        }

        transaction.commit()
    }

    pub fn load_from_db(
        account_id: u32,
        guid: u64,
        world_context: Arc<WorldContext>,
        session: Arc<WorldSession>,
    ) -> Player {
        let conn = world_context.database.characters.get().unwrap();
        let character = CharacterRepository::fetch_basic_character_data(&conn, guid)
            .expect("Failed to load character from DB");

        assert!(
            character.account_id == account_id,
            "Attempt to load a character belonging to another account"
        );

        let guid = ObjectGuid::new(HighGuidType::Player, guid as u32);

        let internal_values = Arc::new(RwLock::new(InternalValues::new(PLAYER_END as usize)));

        // Load inventory BEFORE acquiring internal_values.write() otherwise we deadlock because
        // PlayerInventory::set calls internal_values.write() too
        let mut inventory = PlayerInventory::new(internal_values.clone());
        ItemRepository::load_player_inventory(&conn, guid.raw() as u32)
            .into_iter()
            .for_each(|record| {
                let item = Item::new(
                    record.guid,
                    record.entry,
                    record.owner_guid.unwrap(),
                    record.stack_count,
                    true,
                );

                inventory.set(record.slot, item);
            });

        let mut values = internal_values.write();

        let chr_races_record = world_context
            .data_store
            .get_race_record(character.race as u32)
            .expect("Cannot load character because it has an invalid race id in DB");

        let display_id = if character.gender == Gender::Male as u8 {
            chr_races_record.male_display_id
        } else {
            chr_races_record.female_display_id
        };

        let power_type = world_context
            .data_store
            .get_class_record(character.class as u32)
            .map(|cl| PowerType::n(cl.power_type).unwrap())
            .expect("Cannot load character because it has an invalid class id in DB");

        let spells = CharacterRepository::fetch_character_spells(&conn, guid.raw());

        values.set_guid(ObjectFields::ObjectFieldGuid.into(), &guid);

        let object_type = make_bitflags!(ObjectTypeMask::{Object | Unit | Player}).bits();
        values.set_u32(ObjectFields::ObjectFieldType.into(), object_type);

        values.set_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0);

        values.set_u32(UnitFields::UnitFieldLevel.into(), character.level as u32);
        values.set_u32(
            UnitFields::PlayerFieldMaxLevel.into(),
            world_context.config.world.game.player.maxlevel,
        );

        values.set_u32(UnitFields::PlayerXp.into(), character.experience);
        values.set_u32(
            UnitFields::PlayerNextLevelXp.into(),
            world_context
                .data_store
                .get_player_required_experience_at_level(character.level.into()),
        );

        values.set_u8(UnitFields::UnitFieldBytes0.into(), 0, character.race as u8);

        values.set_u8(UnitFields::UnitFieldBytes0.into(), 1, character.class as u8);

        let gender = Gender::n(character.gender).expect("Character has invalid gender in DB");
        values.set_u8(UnitFields::UnitFieldBytes0.into(), 2, gender as u8);

        values.set_u8(UnitFields::UnitFieldBytes0.into(), 3, power_type as u8);

        values.set_u8(UnitFields::UnitFieldBytes2.into(), 1, 0x28); // UNIT_BYTE2_FLAG_UNK3 | UNIT_BYTE2_FLAG_UNK5

        values.set_u8(
            UnitFields::PlayerBytes.into(),
            0,
            character.visual_features.skin,
        );
        values.set_u8(
            UnitFields::PlayerBytes.into(),
            1,
            character.visual_features.face,
        );
        values.set_u8(
            UnitFields::PlayerBytes.into(),
            2,
            character.visual_features.hairstyle,
        );
        values.set_u8(
            UnitFields::PlayerBytes.into(),
            3,
            character.visual_features.haircolor,
        );
        values.set_u8(
            UnitFields::PlayerBytes2.into(),
            0,
            character.visual_features.facialstyle,
        );
        values.set_u8(UnitFields::PlayerBytes2.into(), 3, 0x02); // Unk
        values.set_u8(UnitFields::PlayerBytes3.into(), 0, gender as u8);

        values.set_u32(UnitFields::UnitFieldDisplayid.into(), display_id);
        values.set_u32(UnitFields::UnitFieldNativedisplayid.into(), display_id);
        values.set_f32(
            UnitFields::UnitFieldBoundingRadius.into(),
            PLAYER_DEFAULT_BOUNDING_RADIUS,
        );
        values.set_f32(
            UnitFields::UnitFieldCombatReach.into(),
            PLAYER_DEFAULT_COMBAT_REACH,
        );

        values.set_u32(UnitFields::PlayerFieldCoinage.into(), character.money);

        // Base attributes
        let base_attributes_record = world_context
            .data_store
            .get_player_base_attributes(character.race, character.class, character.level as u32)
            .expect("unable to retrieve base attributes for this race/class/level combination");
        values.set_u32(
            UnitFields::UnitFieldStat0 as usize + UnitAttribute::Strength as usize,
            base_attributes_record.strength,
        );
        values.set_u32(
            UnitFields::UnitFieldStat0 as usize + UnitAttribute::Agility as usize,
            base_attributes_record.agility,
        );
        values.set_u32(
            UnitFields::UnitFieldStat0 as usize + UnitAttribute::Stamina as usize,
            base_attributes_record.stamina,
        );
        values.set_u32(
            UnitFields::UnitFieldStat0 as usize + UnitAttribute::Intellect as usize,
            base_attributes_record.intellect,
        );
        values.set_u32(
            UnitFields::UnitFieldStat0 as usize + UnitAttribute::Spirit as usize,
            base_attributes_record.spirit,
        );

        // Armor is SpellSchool::Normal resistance
        values.set_u32(
            UnitFields::UnitFieldResistances as usize + SpellSchool::Normal as usize,
            base_attributes_record.agility * 2,
        );

        let base_health_mana_record = world_context
            .data_store
            .get_player_base_health_mana(character.class, character.level as u32)
            .expect("unable to retrieve base health/mana for this class/level combination");

        // Set health
        values.set_u32(UnitFields::UnitFieldHealth.into(), character.current_health);
        values.set_u32(
            UnitFields::UnitFieldBaseHealth.into(),
            base_health_mana_record.base_health,
        );
        // FIXME: calculate max from base + modifiers
        values.set_u32(
            UnitFields::UnitFieldMaxHealth.into(),
            base_health_mana_record.base_health,
        );

        // Set other powers
        for power_type in PowerType::iter().skip(1) {
            // TODO: Save powers in characters table
            values.set_u32(
                UnitFields::UnitFieldPower1 as usize + power_type as usize,
                world_context.data_store.get_player_max_base_power(
                    power_type,
                    character.class,
                    character.level as u32,
                    false,
                ),
            );
            values.set_u32(
                UnitFields::UnitFieldMaxPower1 as usize + power_type as usize,
                world_context.data_store.get_player_max_base_power(
                    power_type,
                    character.class,
                    character.level as u32,
                    false,
                ),
            );
        }

        values.set_u32(
            UnitFields::UnitFieldFactionTemplate.into(),
            chr_races_record.faction_id,
        );

        values.set_i32(UnitFields::PlayerFieldWatchedFactionIndex.into(), -1);

        // Skills
        let skills = CharacterRepository::fetch_character_skills(&conn, guid.raw());
        for (index, skill) in skills.iter().enumerate() {
            values.set_u16(
                UnitFields::PlayerSkillInfo1_1 as usize + (index * 3),
                0,
                skill.skill_id,
            );
            // Note: PlayerSkillInfo1_1 offset 1 is "step"
            values.set_u16(
                UnitFields::PlayerSkillInfo1_1 as usize + 1 + (index * 3),
                0,
                skill.value,
            );
            values.set_u16(
                UnitFields::PlayerSkillInfo1_1 as usize + 1 + (index * 3),
                1,
                skill.max_value,
            );
        }

        values.set_u32(
            UnitFields::UnitFieldFlags.into(),
            UnitFlags::PlayerControlled as u32,
        );

        // Action buttons
        let action_buttons: HashMap<usize, ActionButton> =
            CharacterRepository::fetch_action_buttons(&conn, guid.raw())
                .into_iter()
                .map(|button| (button.position as usize, button))
                .collect();

        // Reputations
        let faction_standings: HashMap<u32, FactionStanding> = {
            let records = CharacterRepository::fetch_faction_standings(&conn, guid.raw());
            let mut result: HashMap<u32, FactionStanding> = HashMap::new();

            for db_record in records {
                if let Some(dbc_record) = world_context
                    .data_store
                    .get_faction_record(db_record.faction_id)
                {
                    if dbc_record.position_in_reputation_list >= 0 {
                        result.insert(
                            dbc_record.position_in_reputation_list as u32,
                            FactionStanding {
                                faction_id: db_record.faction_id,
                                base_standing: dbc_record
                                    .base_reputation_standing(
                                        character.race.into(),
                                        character.class.into(),
                                    )
                                    .unwrap_or(0),
                                db_standing: db_record.standing,
                                flags: db_record.flags,
                                position_in_reputation_list: dbc_record.position_in_reputation_list
                                    as u32,
                            },
                        );
                    } else {
                        warn!("faction with position_in_reputation_list < 0 found in character_reputations");
                    }
                } else {
                    warn!("invalid faction_id in character_reputations");
                }
            }

            result
        };

        let mut quest_statuses = CharacterRepository::load_quest_statuses(&conn, guid.raw());
        for (slot, (quest_id, context)) in quest_statuses.iter_mut().enumerate() {
            if context.status == PlayerQuestStatus::TurnedIn {
                continue;
            }

            if slot >= MAX_QUESTS_IN_LOG {
                error!(
                    "player {} {:?} has more than the maximum number of quests allowed",
                    character.name, guid
                );
            }

            context.slot = Some(slot);

            let quest_template = world_context
                .data_store
                .get_quest_template(*quest_id)
                .unwrap();
            let base_index =
                UnitFields::PlayerQuestLog1_1 as usize + (slot * QUEST_SLOT_OFFSETS_COUNT);

            values.set_u32(base_index, quest_template.entry);

            match context.status {
                PlayerQuestStatus::ObjectivesCompleted => values.set_u32(
                    base_index + QuestSlotOffset::State as usize,
                    QuestSlotState::Completed as u32,
                ),
                PlayerQuestStatus::Failed => values.set_u32(
                    base_index + QuestSlotOffset::State as usize,
                    QuestSlotState::Failed as u32,
                ),
                _ => (),
            }

            // TODO: Restore this from the database
            if let Some(timer) = quest_template
                .time_limit
                .filter(|limit| *limit != Duration::ZERO)
            {
                values.set_u32(
                    base_index + QuestSlotOffset::Timer as usize,
                    (SystemTime::now() + timer)
                        .duration_since(UNIX_EPOCH)
                        .expect("time went backward")
                        .as_millis() as u32,
                );
            }

            for index in 0..MAX_QUEST_OBJECTIVES_COUNT {
                values.set_u8(
                    base_index + QuestSlotOffset::Counters as usize,
                    index,
                    context.entity_counts[index] as u8,
                );
            }
        }

        values.reset_dirty();

        Self {
            session,
            world_context: world_context.clone(),
            guid,
            name: character.name,
            internal_values: internal_values.clone(),
            inventory,
            spells,
            action_buttons,
            faction_standings,
            quest_statuses,
            in_combat_with: RwLock::new(HashSet::new()),
            currently_looting: None,
            partial_regen_period_end: Instant::now(),
        }
    }

    pub fn spells(&self) -> &Vec<u32> {
        &self.spells
    }

    pub fn guid(&self) -> ObjectGuid {
        self.guid
    }

    pub fn action_buttons(&self) -> &HashMap<usize, ActionButton> {
        &self.action_buttons
    }

    pub fn faction_standings(&self) -> &HashMap<u32, FactionStanding> {
        &self.faction_standings
    }

    pub fn build_create_object(
        &self,
        movement: Option<MovementUpdateData>,
        for_self: bool,
    ) -> SmsgCreateObject {
        let mut update_builder = UpdateBlockBuilder::new();
        let internal_values = self.internal_values.read();
        for index in 0..PLAYER_END {
            let value = internal_values.get_u32(index as usize);
            if value != 0 {
                update_builder.add(index as usize, value);
            }
        }
        drop(internal_values);

        let blocks = update_builder.build();
        let flags = if for_self {
            make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition | SelfUpdate})
        } else {
            make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition})
        };

        let mut update_data = vec![CreateData {
            update_type: UpdateType::CreateObject2,
            packed_guid: self.guid.as_packed(),
            object_type: ObjectTypeId::Player,
            flags,
            movement,
            low_guid_part: None,
            high_guid_part: Some(HighGuidType::Player as u32),
            blocks,
        }];

        if for_self {
            let inventory_updates: Vec<CreateData> = self.inventory.build_create_data();
            update_data.extend(inventory_updates);
        }

        SmsgCreateObject {
            updates_count: update_data.len() as u32,
            has_transport: false,
            updates: update_data,
        }
    }

    pub fn can_start_quest(&self, quest_template: &QuestTemplate) -> bool {
        self.check_quest_requirements(quest_template).is_none()
    }

    /**
     * Checks to perform:
     *
     * - (1) player does not have the quest (OK)
     * - (2) satisfy exclusive_group requirements (TODO)
     * - (3) player class is in required_classes mask (OK)
     * - (4) player race is in required_races mask (OK)
     * - (5) player level >= quest_template.min_level (OK)
     * - (6) player skill level >= quest_template required skill (TODO)
     * - (7) player reputation >= quest_template required reputation (TODO)
     * - (8) player has done the previous quest (previous_quest_id > 0) (OK)
     * - (9) player is actively doing the parent quest (previous_quest_id < 0) (TODO)
     * - (10) player can only have one timed quest at the same time (TODO)
     * - (11) player is not doing/has not done the next quest in chain (TODO)
     * - (12) player has done the previous quest in chain (see qInfo.prevChainQuests in MaNGOS) (TODO)
     * - (13) player still has daily quests allowance if quest is daily (TODO)
     * - (14) game event must be active if quest is related to one (TODO)
     */
    fn check_quest_requirements(&self, quest_template: &QuestTemplate) -> Option<QuestStartError> {
        // (1)
        if self.quest_statuses.contains_key(&quest_template.entry) {
            return Some(QuestStartError::AlreadyOn);
        }

        {
            let values = self.internal_values.read();

            // (3)
            let class_id = values.get_u8(UnitFields::UnitFieldBytes0.into(), 1);
            if !quest_template.required_classes.is_empty()
                && !quest_template
                    .required_classes
                    .contains(CharacterClassBit::from(class_id))
            {
                return Some(QuestStartError::FailedRequirement);
            }

            // (4)
            let race_id = values.get_u8(UnitFields::UnitFieldBytes0.into(), 0);
            if !quest_template.required_races.is_empty()
                && !quest_template
                    .required_races
                    .contains(CharacterRaceBit::from(race_id))
            {
                return Some(QuestStartError::WrongRace);
            }

            // (5)
            if values.get_u32(UnitFields::UnitFieldLevel.into()) < quest_template.min_level {
                return Some(QuestStartError::TooLowLevel);
            }
        }

        // (8)
        if let Some(previous_quest) = quest_template.previous_quest_id() {
            match self.quest_statuses.get(&previous_quest) {
                Some(context) if context.status == PlayerQuestStatus::TurnedIn => (),
                _ => return Some(QuestStartError::FailedRequirement),
            }
        }

        None
    }

    pub fn quest_status(&self, quest_id: &u32) -> Option<&QuestLogContext> {
        self.quest_statuses.get(quest_id)
    }

    pub fn quest_statuses(&self) -> &HashMap<u32, QuestLogContext> {
        &self.quest_statuses
    }

    pub fn can_turn_in_quest(&self, quest_id: &u32) -> bool {
        self.quest_status(quest_id)
            .is_some_and(|ctx| ctx.status == PlayerQuestStatus::ObjectivesCompleted)
    }

    pub fn is_progressing_quest(&self, quest_id: &u32) -> bool {
        self.quest_status(quest_id)
            .is_some_and(|ctx| ctx.status == PlayerQuestStatus::InProgress)
    }

    pub fn start_quest(&mut self, quest_template: &QuestTemplate) {
        if !self.can_start_quest(&quest_template) {
            error!("attempt to start a quest that the player cannot start");
            return;
        }

        #[allow(unused_assignments)]
        let mut quest_added = false;
        {
            let mut first_empty_slot: Option<usize> = None;
            let mut values_guard = self.internal_values.write();
            for i in 0..MAX_QUESTS_IN_LOG {
                let quest_id_in_slots = values_guard.get_u32(
                    UnitFields::PlayerQuestLog1_1 as usize + (i * QUEST_SLOT_OFFSETS_COUNT),
                );
                if quest_id_in_slots == 0 {
                    first_empty_slot = Some(i);
                    break;
                }
            }

            if let Some(slot) = first_empty_slot {
                let base_index =
                    UnitFields::PlayerQuestLog1_1 as usize + (slot * QUEST_SLOT_OFFSETS_COUNT);

                values_guard.set_u32(base_index, quest_template.entry);

                if let Some(timer) = quest_template
                    .time_limit
                    .filter(|limit| *limit != Duration::ZERO)
                {
                    values_guard.set_u32(
                        base_index + QuestSlotOffset::Timer as usize,
                        (SystemTime::now() + timer)
                            .duration_since(UNIX_EPOCH)
                            .expect("time went backward")
                            .as_millis() as u32,
                    );
                }

                self.quest_statuses.insert(
                    quest_template.entry,
                    QuestLogContext {
                        slot: Some(slot),
                        status: PlayerQuestStatus::InProgress,
                        entity_counts: [0, 0, 0, 0],
                    },
                );

                quest_added = true;
            } else {
                error!("player quest log is full");
                return;
            }
        }

        if quest_added {
            self.try_complete_quest(quest_template);
        }
    }

    pub fn remove_quest(&mut self, slot_to_remove: usize) {
        self.quest_statuses.retain(|_, context| match context.slot {
            None => true,
            Some(slot) => slot != slot_to_remove,
        });

        let mut values_guard = self.internal_values.write();
        let base_index =
            UnitFields::PlayerQuestLog1_1 as usize + (slot_to_remove * QUEST_SLOT_OFFSETS_COUNT);
        for index in 0..QUEST_SLOT_OFFSETS_COUNT {
            values_guard.set_u32(base_index + index, 0);
        }
    }

    pub fn try_complete_quest(&mut self, quest_template: &QuestTemplate) {
        let quest_id = quest_template.entry;
        if let Some(context) = self.quest_statuses.get_mut(&quest_id) {
            if context.status != PlayerQuestStatus::InProgress || context.slot.is_none() {
                return;
            }

            for index in 0..MAX_QUEST_OBJECTIVES_COUNT {
                let current_entity_count = context.entity_counts[index];
                let objective_entity_count = quest_template.required_entity_counts[index];

                if current_entity_count < objective_entity_count {
                    return;
                }

                let required_item_id = quest_template.required_item_ids[index];
                let required_item_count = quest_template.required_item_counts[index];

                if self.inventory.get_item_count(required_item_id) < required_item_count {
                    return;
                }
            }

            // TODO: Check exploration etc

            context.status = PlayerQuestStatus::ObjectivesCompleted;
            let mut values_guard = self.internal_values.write();
            let base_index = UnitFields::PlayerQuestLog1_1 as usize
                + (context.slot.unwrap() * QUEST_SLOT_OFFSETS_COUNT);
            values_guard.set_u32(
                base_index + QuestSlotOffset::State as usize,
                QuestSlotState::Completed as u32,
            )
        }
    }

    pub fn reward_quest(
        &mut self,
        quest_id: u32,
        chosen_reward_index: u32,
        data_store: Arc<DataStore>,
    ) -> Option<u32> {
        warn!("TODO: Implement Player::reward_quest (reputation, ...)");

        if let Some(context) = self.quest_statuses.get_mut(&quest_id) {
            if let Some(quest_template) = data_store.get_quest_template(quest_id) {
                if context.status != PlayerQuestStatus::ObjectivesCompleted {
                    error!(
                        "attempt to reward a quest with an unexpected status {:?}",
                        context.status
                    );
                    return None;
                }

                context.status = PlayerQuestStatus::TurnedIn;

                {
                    let mut values_guard = self.internal_values.write();
                    let base_index = UnitFields::PlayerQuestLog1_1 as usize
                        + (context.slot.unwrap() * QUEST_SLOT_OFFSETS_COUNT);

                    for index in 0..QUEST_SLOT_OFFSETS_COUNT {
                        values_guard.set_u32(base_index + index, 0);
                    }
                }

                context.slot = None;

                self.modify_money(quest_template.required_or_reward_money);

                match quest_template.reward_choice_items()[chosen_reward_index as usize] {
                    (0, _) | (_, 0) => (),
                    (id, count) => {
                        self.auto_store_new_item(id, count).unwrap();
                        ()
                    }
                }

                for (id, count) in quest_template
                    .reward_items()
                    .into_iter()
                    .filter(|(id, count)| *id != 0 && *count != 0)
                {
                    self.auto_store_new_item(id, count).unwrap();
                }

                let xp = quest_template.experience_reward_at_level(self.level());
                self.give_experience(xp, None);
                return Some(xp);
            }
        }

        error!("attempt to reward an non-existing quest");
        None
    }

    pub fn set_in_combat_with(&self, guid: ObjectGuid) {
        self.in_combat_with.write().insert(guid);
    }

    pub fn reset_in_combat_with(&self) {
        self.in_combat_with.write().clear();
    }

    pub fn unset_in_combat_with(&self, guid: ObjectGuid) {
        self.in_combat_with.write().remove(&guid);
    }

    pub fn in_combat_with(&self) -> HashSet<ObjectGuid> {
        self.in_combat_with.read().clone()
    }

    pub fn is_in_combat_with(&self, other: &ObjectGuid) -> bool {
        self.in_combat_with.read().contains(other)
    }

    // NOTE: MaNGOS uses f32 for internal calculation but client expects u32
    pub fn attribute(&self, attr: UnitAttribute) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldStat0 as usize + attr as usize)
    }

    pub fn resistance(&self, spell_school: SpellSchool) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldResistances as usize + spell_school as usize)
    }

    pub fn armor(&self) -> u32 {
        self.resistance(SpellSchool::Normal)
    }

    pub fn base_attack_time(
        &self,
        attack_type: WeaponAttackType,
        data_store: Arc<DataStore>,
    ) -> Duration {
        let slot = match attack_type {
            WeaponAttackType::MainHand => InventorySlot::EquipmentMainHand,
            WeaponAttackType::OffHand => InventorySlot::EquipmentOffHand,
            WeaponAttackType::Ranged => InventorySlot::EquipmentRanged,
        } as u32;

        self.inventory
            .get(slot)
            .and_then(|item| {
                data_store
                    .get_item_template(item.entry())
                    .map(|template| Duration::from_millis(template.delay as u64))
            })
            .unwrap_or(BASE_ATTACK_TIME)
    }

    pub fn base_damage(
        &self,
        attack_type: WeaponAttackType,
        data_store: Arc<DataStore>,
    ) -> ValueRange<f32> {
        let slot = match attack_type {
            WeaponAttackType::MainHand => InventorySlot::EquipmentMainHand,
            WeaponAttackType::OffHand => InventorySlot::EquipmentOffHand,
            WeaponAttackType::Ranged => InventorySlot::EquipmentRanged,
        } as u32;

        self.inventory
            .get(slot)
            .and_then(|item| {
                data_store.get_item_template(item.entry()).map(|template| {
                    let min = template
                        .damages
                        .iter()
                        .map(|dmg| dmg.damage_min)
                        .sum::<f32>();

                    let max = template
                        .damages
                        .iter()
                        .map(|dmg| dmg.damage_max)
                        .sum::<f32>();

                    ValueRange::new(min, max)
                })
            })
            .unwrap_or(ValueRange::new(BASE_DAMAGE, BASE_DAMAGE))
    }

    pub fn notify_killed_creature(&mut self, creature_guid: &ObjectGuid, creature_entry: u32) {
        // Update quest kills counters
        let mut updated_quests: Vec<QuestTemplate> = Vec::new();
        self.quest_statuses.iter_mut().for_each(|(quest_id, ctx)| {
            let quest_template = self
                .world_context
                .data_store
                .get_quest_template(*quest_id)
                .expect("player has non-existing quest in log");

            if let Some((objective_index, required_count)) =
                quest_template.creature_requirements(creature_entry)
            {
                match (ctx.status, ctx.slot) {
                    (PlayerQuestStatus::InProgress, Some(slot)) => {
                        let current_count = ctx.entity_counts[objective_index];
                        if current_count < required_count {
                            let new_count = (current_count + 1).min(required_count);
                            ctx.entity_counts[objective_index] = new_count;

                            {
                                let mut values_guard = self.internal_values.write();
                                let index = UnitFields::PlayerQuestLog1_1 as usize
                                    + (slot * QUEST_SLOT_OFFSETS_COUNT
                                        + QuestSlotOffset::Counters as usize);

                                values_guard.set_u8(index, objective_index, new_count as u8);
                            }

                            let packet = ServerMessage::new(SmsgQuestUpdateAddKill {
                                quest_id: quest_template.entry,
                                entity_id: creature_entry,
                                new_count,
                                required_count,
                                guid: creature_guid.raw(),
                            });

                            self.session.send(&packet).unwrap();

                            updated_quests.push(quest_template.clone());
                        }
                    }
                    _ => (),
                }
            }
        });

        // Try to complete the affected quests
        for quest_template in updated_quests {
            self.try_complete_quest(&quest_template);
        }
    }

    pub fn money(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::PlayerFieldCoinage.into())
    }

    pub fn modify_money(&self, amount: i32) {
        let current_money = self.money();
        let new_money = current_money.saturating_add_signed(amount);
        self.internal_values
            .write()
            .set_u32(UnitFields::PlayerFieldCoinage.into(), new_money);
    }

    pub fn set_has_cast_recently(&mut self) {
        self.partial_regen_period_end = Instant::now() + Duration::from_secs(5);
    }

    // Assume that we store in bags for now (TODO bank later)
    pub fn auto_store_new_item(
        &mut self,
        item_id: u32,
        stack_count: u32,
    ) -> Result<u32, InventoryResult> {
        let item_template = self
            .world_context
            .data_store
            .get_item_template(item_id)
            .expect("unknown item found in inventory");

        let mut remaining_stack_count = stack_count;

        // Attempt to distribute the new stacks over existing incomplete stacks
        // TODO: Implement bags, we only check in the backpack for now
        for slot in InventorySlot::BACKPACK_START..InventorySlot::BACKPACK_END {
            if let Some(item) = self.inventory.get_mut(slot) {
                if item.entry() == item_id && item.stack_count() < item_template.max_stack_count {
                    let available_stack_count = item_template.max_stack_count - item.stack_count();
                    let stack_count_to_add = remaining_stack_count.min(available_stack_count);
                    item.change_stack_count(stack_count_to_add.try_into().unwrap());
                    remaining_stack_count = remaining_stack_count - stack_count_to_add;
                }
            }

            if remaining_stack_count == 0 {
                break;
            }
        }

        // Drop the leftover in empty slots
        let mut chosen_slot = u32::MAX;
        while remaining_stack_count > 0 {
            match self.inventory.find_first_free_slot() {
                Some(slot) => {
                    let item_guid: u32 = self.world_context.next_item_guid();
                    let stack_count_to_add =
                        remaining_stack_count.min(item_template.max_stack_count);
                    let item = Item::new(
                        item_guid,
                        item_id,
                        self.guid.raw(),
                        stack_count_to_add,
                        false,
                    );
                    remaining_stack_count = remaining_stack_count - stack_count_to_add;

                    let packet = ServerMessage::new(SmsgCreateObject {
                        updates_count: 1,
                        has_transport: false,
                        updates: vec![item.build_create_data()],
                    });

                    self.inventory.set(slot, item);
                    self.session.send(&packet).unwrap();

                    chosen_slot = slot;
                }
                None => return Err(InventoryResult::InventoryFull),
            }
        }

        let total_count = self.inventory.get_item_count(item_id);
        let packet = ServerMessage::new(SmsgItemPushResult {
            player_guid: self.guid.clone(),
            loot_source: 0,
            is_created: 0,
            is_visible_in_chat: 1,
            bag_slot: 255, // FIXME: INVENTORY_SLOT_BAG_0
            item_slot: chosen_slot,
            item_id,
            item_suffix_factor: 0,      // FIXME
            item_random_property_id: 0, // FIXME
            count: stack_count,
            total_count_of_this_item_in_inventory: total_count,
        });
        self.session.send(&packet).unwrap();

        // Try to complete in-progress quests
        let quest_ids: Vec<u32> = self
            .quest_statuses
            .iter()
            .filter_map(|(&quest_id, context)| {
                if context.status == PlayerQuestStatus::InProgress {
                    Some(quest_id)
                } else {
                    None
                }
            })
            .collect();
        let world_context_local_clone = self.world_context.clone();
        for quest_id in quest_ids {
            let Some(&ref quest_template) = world_context_local_clone
                .data_store
                .get_quest_template(quest_id)
            else {
                continue;
            };

            self.try_complete_quest(&quest_template);
        }

        Ok(chosen_slot)
    }

    pub fn remove_item(&mut self, slot: u32) -> Option<Item> {
        self.inventory.remove(slot).or_else(|| {
            error!("Player::remove_item: no item found in slot {slot}");
            None
        })

        // TODO: recalculate quest status (potentially back from ObjectivesCompleted to InProgress)
    }

    pub fn get_inventory_item(&self, slot: u32) -> Option<&Item> {
        self.inventory.get(slot)
    }

    pub fn try_equip_item_from_inventory(&mut self, from_slot: u32) -> InventoryResult {
        let Some(item_to_equip) = self.inventory.get(from_slot) else {
            return InventoryResult::SlotIsEmpty;
        };

        let Some(item_template) = self
            .world_context
            .data_store
            .get_item_template(item_to_equip.entry())
        else {
            return InventoryResult::ItemNotFound;
        };

        let Some(destination_slot) = self.inventory.equipment_slot_for(item_template) else {
            return InventoryResult::ItemCantBeEquipped;
        };
        let destination_slot = destination_slot as u32;

        if self.inventory.has_item_in_slot(destination_slot) {
            self.inventory.swap(from_slot, destination_slot);
        } else {
            self.inventory.move_item(from_slot, destination_slot);
        }

        InventoryResult::Ok
    }

    pub fn try_swap_inventory_item(&mut self, from_slot: u32, to_slot: u32) -> InventoryResult {
        let (maybe_moved_item, maybe_target_item) = self.inventory.get2_mut(from_slot, to_slot);

        // There's no item in from_slot (cheating player?)
        let Some(moved_item) = maybe_moved_item else {
            return InventoryResult::SlotIsEmpty;
        };
        let moved_item_entry = moved_item.entry();
        let moved_item_stack_count = moved_item.stack_count();

        let Some(moved_item_template) = self
            .world_context
            .data_store
            .get_item_template(moved_item.entry())
        else {
            return InventoryResult::ItemNotFound;
        };

        let is_destination_gear_slot =
            to_slot >= InventorySlot::EQUIPMENT_START && to_slot < InventorySlot::EQUIPMENT_END;
        let is_origin_gear_slot =
            from_slot >= InventorySlot::EQUIPMENT_START && from_slot < InventorySlot::EQUIPMENT_END;

        // Equipment is dragged over a gear slot
        // => Check that moved_item can go in to_slot
        if is_destination_gear_slot {
            let allowed_gear_slots: Vec<u32> = moved_item_template
                .allowed_gear_slots()
                .into_iter()
                .map(|slot| slot as u32)
                .collect();
            if !allowed_gear_slots.contains(&to_slot) {
                return InventoryResult::ItemDoesntGoToSlot;
            }
        }

        if let Some(target_item) = maybe_target_item {
            let target_item_entry = target_item.entry();
            let target_item_stack_count = target_item.stack_count();
            let Some(target_item_template) = self
                .world_context
                .data_store
                .get_item_template(target_item.entry())
            else {
                return InventoryResult::ItemNotFound;
            };

            // Equipment is dragged from gear onto another gear piece in a bag
            // => Check that target_item can go in from_slot
            if is_origin_gear_slot {
                let allowed_gear_slots: Vec<u32> = target_item_template
                    .allowed_gear_slots()
                    .into_iter()
                    .map(|slot| slot as u32)
                    .collect();
                if !allowed_gear_slots.contains(&from_slot) {
                    return InventoryResult::ItemDoesntGoToSlot;
                }
            }

            // If both items are the same template and target still has available stack space,
            // recalculate both stacks
            if moved_item_entry == target_item_entry
                && target_item_stack_count < target_item_template.max_stack_count
            {
                let stack_diff = (target_item_template.max_stack_count - target_item_stack_count)
                    .min(moved_item_stack_count) as i32;

                target_item.change_stack_count(stack_diff);
                if stack_diff < moved_item_stack_count as i32 {
                    moved_item.change_stack_count(stack_diff as i32 * -1);
                } else {
                    // If the moved item has no stack after the transfer, delete it
                    self.remove_item(from_slot);
                }

                return InventoryResult::Ok;
            }

            self.inventory.swap(from_slot, to_slot);
            InventoryResult::Ok
        } else {
            if is_destination_gear_slot {
                // Moving the item from a bag to gear: equip it
                self.try_equip_item_from_inventory(from_slot)
            } else {
                // Moving the item to a bag (from gear or a bag): just move it
                self.inventory.move_item(from_slot, to_slot);
                InventoryResult::Ok
            }
        }
    }

    pub fn try_split_item(
        &mut self,
        from_slot: u32,
        destination_slot: u32,
        count: u8,
    ) -> InventoryResult {
        match self.inventory.get2_mut(from_slot, destination_slot) {
            (None, _) => {
                // There's no item in from_slot (cheating player?)
                InventoryResult::SlotIsEmpty
            }
            (Some(moved_item), _) if moved_item.stack_count() <= count.into() => {
                InventoryResult::CouldntSplitItems
            }
            (Some(moved_item), None) => {
                // Dropping the extra stacks on an empty slot
                moved_item.change_stack_count(count as i32 * -1);
                let new_item_guid: u32 = self.world_context.next_item_guid();
                let new_item = Item::new(
                    new_item_guid,
                    moved_item.entry(),
                    self.guid.raw(),
                    count.into(),
                    false,
                );
                let packet = ServerMessage::new(SmsgCreateObject {
                    updates_count: 1,
                    has_transport: false,
                    updates: vec![new_item.build_create_data()],
                });

                self.inventory.set(destination_slot, new_item);
                self.session.send(&packet).unwrap();

                InventoryResult::Ok
            }
            (Some(moved_item), Some(target_item)) if moved_item.entry() != target_item.entry() => {
                // Dropping the extra stacks on another item
                InventoryResult::CouldntSplitItems
            }
            (Some(moved_item), Some(target_item)) => {
                // Dropping the extra stacks on the same item
                let Some(target_item_template) = self
                    .world_context
                    .data_store
                    .get_item_template(target_item.entry())
                else {
                    return InventoryResult::ItemNotFound;
                };
                let available_stack_count =
                    target_item_template.max_stack_count - target_item.stack_count();
                let stacks_to_move = available_stack_count.min(count.into()) as i32;

                moved_item.change_stack_count(stacks_to_move * -1);
                target_item.change_stack_count(stacks_to_move);
                InventoryResult::Ok
            }
        }
    }

    pub fn inventory(&self) -> &PlayerInventory {
        &self.inventory
    }

    pub fn inventory_mut(&mut self) -> &mut PlayerInventory {
        &mut self.inventory
    }

    pub fn get_inventory_updates_and_reset(&mut self) -> Vec<UpdateData> {
        self.inventory.list_updated_and_reset()
    }

    pub fn set_looting(&mut self, entity_id: Option<EntityId>) {
        match (self.currently_looting, entity_id) {
            (Some(id1), Some(id2)) if id1 != id2 => {
                warn!("Player::set_looting called but player is already looting another entity")
            }
            _ => (),
        }

        self.currently_looting = entity_id;
    }

    pub fn currently_looting(&self) -> Option<EntityId> {
        self.currently_looting
    }

    pub fn health_regen_per_tick(&self) -> f32 {
        let level = self.level().min(GAME_TABLE_MAX_LEVEL);
        let class = self
            .internal_values
            .read()
            .get_u8(UnitFields::UnitFieldBytes0.into(), 0) as u32;
        let spirit_stat = self
            .internal_values
            .read()
            .get_u32(UnitFields::UnitFieldStat0 as usize + UnitAttribute::Spirit as usize)
            as f32;
        let base_spirit = spirit_stat.min(50.0);
        let extra_spirit = spirit_stat - base_spirit;

        let index = ((class - 1) * GAME_TABLE_MAX_LEVEL + level - 1) as usize;
        let maybe_base_regen_hp_record = self.world_context.data_store.get_gtOCTRegenHP(index);
        let maybe_regen_hp_from_spirit_record =
            self.world_context.data_store.get_gtRegenHPPerSpt(index);

        match (
            maybe_base_regen_hp_record,
            maybe_regen_hp_from_spirit_record,
        ) {
            (None, _) | (_, None) => 0.0,
            (Some(base_record), Some(from_spirit_record)) => {
                base_spirit * base_record.ratio + extra_spirit * from_spirit_record.ratio
            }
        }
    }

    pub fn mana_regen_per_tick(&self) -> f32 {
        let has_cast_recently = Instant::now() >= self.partial_regen_period_end;
        let values_index = if has_cast_recently {
            UnitFields::PlayerFieldModManaRegenInterrupt
        } else {
            UnitFields::PlayerFieldModManaRegen
        };
        let mana_regen = self.internal_values.read().get_f32(values_index as usize);

        mana_regen * 2.
    }

    pub fn energy_regen_per_tick(&self) -> f32 {
        // TODO: Use SPELL_AURA_MOD_POWER_REGEN_PERCENT
        20.
    }

    pub fn rage_degen_per_tick(&self) -> f32 {
        // TODO: Use SPELL_AURA_MOD_POWER_REGEN_PERCENT
        20.
    }

    pub fn calculate_mana_regen(&self) {
        // TODO: Incomplete, see Player::UpdateManaRegen() in MaNGOS
        let intellect = self.attribute(UnitAttribute::Intellect) as f32;

        let level = self.level().min(GAME_TABLE_MAX_LEVEL);
        let class = self
            .internal_values
            .read()
            .get_u8(UnitFields::UnitFieldBytes0.into(), 0) as u32;
        let index = ((class - 1) * GAME_TABLE_MAX_LEVEL + level - 1) as usize;

        let regen_per_spirit = self
            .world_context
            .data_store
            .get_gtRegenHPPerSpt(index)
            .map(|record| self.attribute(UnitAttribute::Spirit) as f32 * record.ratio)
            .unwrap_or(0.);
        let regen_from_stats = intellect.sqrt() * regen_per_spirit;
        let regen_under_fsr = 100.; // TODO: Implement Auras

        {
            let mut values = self.internal_values.write();
            values.set_f32(
                UnitFields::PlayerFieldModManaRegenInterrupt.into(),
                regen_from_stats * regen_under_fsr / 100.,
            );
            values.set_f32(UnitFields::PlayerFieldModManaRegen.into(), regen_from_stats);
        }
    }
}

pub struct PlayerVisualFeatures {
    pub haircolor: u8,
    pub hairstyle: u8,
    pub face: u8,
    pub skin: u8,
    pub facialstyle: u8,
}
