use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use enumflags2::make_bitflags;
use log::{error, warn};
use parking_lot::RwLock;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Error;
use shipyard::Component;
use strum::IntoEnumIterator;

use crate::{
    datastore::{
        data_types::{PlayerCreatePosition, QuestTemplate},
        DataStore,
    },
    entities::player::player_data::FactionStanding,
    game::{value_range::ValueRange, world_context::WorldContext},
    protocol::{
        packets::{CmsgCharCreate, SmsgCreateObject, SmsgLogXpGain},
        server::ServerMessage,
    },
    repositories::{character::CharacterRepository, item::ItemRepository},
    session::world_session::WorldSession,
    shared::constants::{
        AbilityLearnType, CharacterClass, CharacterClassBit, CharacterRaceBit, Gender,
        HighGuidType, InventorySlot, InventoryType, ItemClass, ItemSubclassConsumable,
        ObjectTypeId, ObjectTypeMask, PlayerQuestStatus, PowerType, QuestSlotState,
        QuestStartError, SkillRangeType, SpellSchool, UnitAttribute, UnitFlags, WeaponAttackType,
        BASE_ATTACK_TIME, BASE_DAMAGE, MAX_QUESTS_IN_LOG, MAX_QUEST_OBJECTIVES_COUNT,
        PLAYER_DEFAULT_BOUNDING_RADIUS, PLAYER_DEFAULT_COMBAT_REACH,
    },
};

use self::player_data::{ActionButton, QuestLogContext};

use super::{
    internal_values::{InternalValues, QuestSlotOffset, QUEST_SLOT_OFFSETS_COUNT},
    item::Item,
    object_guid::ObjectGuid,
    update::{CreateData, MovementUpdateData, UpdateBlockBuilder, UpdateFlag, UpdateType},
    update_fields::*,
};

pub mod player_data;

pub type PlayerInventory = HashMap<u32, Item>; // Key is slot

#[derive(Component)]
pub struct Player {
    pub session: Arc<WorldSession>,
    guid: ObjectGuid,
    pub name: String,
    pub internal_values: Arc<RwLock<InternalValues>>,
    inventory: PlayerInventory,
    spells: Vec<u32>,
    action_buttons: HashMap<usize, ActionButton>,
    faction_standings: HashMap<u32, FactionStanding>,
    quest_statuses: HashMap<u32, QuestLogContext>,
    in_combat_with: RwLock<HashSet<ObjectGuid>>,
}

impl Player {
    pub fn create_in_db(
        conn: &mut PooledConnection<SqliteConnectionManager>,
        creation_payload: &CmsgCharCreate,
        account_id: u32,
        data_store: Arc<DataStore>,
    ) -> Result<(), Error> {
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
                let item_guid = ItemRepository::create(&transaction, start_item.id, stack_count);

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

        let mut values = InternalValues::new(PLAYER_END as usize);

        assert!(
            character.account_id == account_id,
            "Attempt to load a character belonging to another account"
        );

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

        let spells = CharacterRepository::fetch_character_spells(&conn, guid);

        let guid = ObjectGuid::new(HighGuidType::Player, guid as u32);
        values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());

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

        // Skip health
        for (index, power_type) in PowerType::iter().enumerate().skip(1) {
            // TODO: Save powers in characters table
            values.set_u32(
                UnitFields::UnitFieldPower1 as usize + index - 1,
                world_context.data_store.get_player_max_base_power(
                    power_type,
                    character.class,
                    character.level as u32,
                    false,
                ),
            );
            values.set_u32(
                UnitFields::UnitFieldMaxPower1 as usize + index - 1,
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

        let inventory: HashMap<u32, Item> =
            ItemRepository::load_player_inventory(&conn, guid.raw() as u32)
                .into_iter()
                .map(|record| {
                    let item = Item::new(
                        record.guid,
                        record.entry,
                        record.owner_guid.unwrap(),
                        record.stack_count,
                    );
                    values.set_u64(
                        UnitFields::PlayerFieldInvSlotHead as usize + (2 * record.slot) as usize,
                        item.guid().raw(),
                    );

                    // Visible bits
                    if record.slot >= InventorySlot::EQUIPMENT_START
                        && record.slot < InventorySlot::EQUIPMENT_END
                    {
                        values.set_u32(
                            UnitFields::PlayerVisibleItem1_0 as usize
                                + (record.slot * MAX_PLAYER_VISIBLE_ITEM_OFFSET) as usize,
                            item.entry(),
                        );
                    }

                    (record.slot, item)
                })
                .collect();

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
            guid,
            name: character.name,
            internal_values: Arc::new(RwLock::new(values)),
            inventory,
            spells,
            action_buttons,
            faction_standings,
            quest_statuses,
            in_combat_with: RwLock::new(HashSet::new()),
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
            let inventory_updates: Vec<CreateData> = self
                .inventory
                .iter()
                .map(|item| item.1.build_create_data())
                .collect();

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
     * - player does not have the quest (OK)
     * - satisfy exclusive_group requirements (TODO)
     * - player class is in required_classes mask (OK)
     * - player race is in required_races mask (OK)
     * - player level >= quest_template.min_level (OK)
     * - player skill level >= quest_template required skill (TODO)
     * - player reputation >= quest_template required reputation (TODO)
     * - player has done the previous quests (see qInfo.prevQuests in MaNGOS) (TODO)
     * - player can only have one timed quest at the same time (TODO)
     * - player is not doing/has not done the next quest in chain (TODO)
     * - player has done the previous quest in chain (see qInfo.prevChainQuests in MaNGOS) (TODO)
     * - player still has daily quests allowance if quest is daily (TODO)
     * - game event must be active if quest is related to one (TODO)
     */
    fn check_quest_requirements(&self, quest_template: &QuestTemplate) -> Option<QuestStartError> {
        if self.quest_statuses.contains_key(&quest_template.entry) {
            return Some(QuestStartError::AlreadyOn);
        }

        {
            let values = self.internal_values.read();

            if values.get_u32(UnitFields::UnitFieldLevel.into()) < quest_template.min_level {
                return Some(QuestStartError::TooLowLevel);
            }

            let race_id = values.get_u8(UnitFields::UnitFieldBytes0.into(), 0);
            if !quest_template.required_races.is_empty()
                && !quest_template
                    .required_races
                    .contains(CharacterRaceBit::from(race_id))
            {
                return Some(QuestStartError::WrongRace);
            }

            let class_id = values.get_u8(UnitFields::UnitFieldBytes0.into(), 1);
            if !quest_template.required_classes.is_empty()
                && !quest_template
                    .required_classes
                    .contains(CharacterClassBit::from(class_id))
            {
                return Some(QuestStartError::FailedRequirement);
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

                // TODO: Check items too
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

    pub fn reward_quest(&mut self, quest_id: u32, data_store: Arc<DataStore>) -> Option<u32> {
        warn!("TODO: Implement Player::reward_quest");

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

                let xp = quest_template.experience_reward_at_level(self.level());
                self.give_experience(xp, None);
                return Some(xp);
            }
        }

        error!("attempt to reward an non-existing quest");
        None
    }

    pub fn level(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::UnitFieldLevel.into())
    }

    pub fn set_in_combat_with(&self, guid: ObjectGuid) {
        self.in_combat_with.write().insert(guid);
    }

    pub fn unset_in_combat_with(&self, guid: ObjectGuid) {
        self.in_combat_with.write().remove(&guid);
    }

    pub fn in_combat_with(&self) -> HashSet<ObjectGuid> {
        self.in_combat_with.read().clone()
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
            .get(&slot)
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
            .get(&slot)
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

    pub fn experience(&self) -> u32 {
        self.internal_values
            .read()
            .get_u32(UnitFields::PlayerXp.into())
    }

    pub fn give_experience(&self, xp: u32, victim_guid: Option<ObjectGuid>) {
        let current_xp = self.experience();

        self.internal_values
            .write()
            .set_u32(UnitFields::PlayerXp.into(), current_xp + xp);

        // TODO: Implement SmsgLogXpGain::new
        let packet = ServerMessage::new(SmsgLogXpGain {
            victim_guid: victim_guid.map(|g| g.raw()).unwrap_or(0),
            given_xp: xp,
            from_kill: victim_guid.is_some(),
            xp_without_rested_bonus: victim_guid.map(|_| xp), // TODO: Implement rested xp
            group_bonus: victim_guid.map(|_| 0),              // TODO: Implement groups
            unk: 0,
        });

        self.session.send(&packet).unwrap();
    }
}

pub struct PlayerVisualFeatures {
    pub haircolor: u8,
    pub hairstyle: u8,
    pub face: u8,
    pub skin: u8,
    pub facialstyle: u8,
}
