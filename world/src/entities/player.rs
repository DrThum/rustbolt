use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use enumflags2::make_bitflags;
use log::{error, warn};
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, Error, Transaction};

use crate::{
    datastore::{data_types::PlayerCreatePosition, DataStore},
    entities::player::player_data::FactionStanding,
    game::{map_manager::MapKey, world_context::WorldContext},
    protocol::{
        packets::{CmsgCharCreate, SmsgAttackerStateUpdate},
        server::ServerMessage,
    },
    repositories::{character::CharacterRepository, item::ItemRepository},
    shared::constants::{
        AbilityLearnType, CharacterClass, CharacterClassBit, CharacterRace, CharacterRaceBit,
        Gender, HighGuidType, InventorySlot, InventoryType, ItemClass, ItemSubclassConsumable,
        ObjectTypeId, ObjectTypeMask, PowerType, SheathState, SkillRangeType, UnitFlags,
        UnitStandState, WeaponAttackType, NUMBER_WEAPON_ATTACK_TYPES,
    },
};

use self::player_data::ActionButton;

use super::{
    internal_values::InternalValues,
    item::Item,
    object_guid::ObjectGuid,
    position::{Position, WorldPosition},
    update::{
        CreateData, MovementUpdateData, UpdateBlock, UpdateBlockBuilder, UpdateData, UpdateFlag,
        UpdateType, WorldEntity,
    },
    update_fields::*,
};

pub mod player_data;

pub type PlayerInventory = HashMap<u32, Item>; // Key is slot

pub struct Player {
    guid: Option<ObjectGuid>,
    name: String,
    values: InternalValues,
    map_key: Option<MapKey>,
    position: Option<WorldPosition>,
    inventory: PlayerInventory,
    spells: Vec<u32>,
    action_buttons: HashMap<usize, ActionButton>,
    faction_standings: HashMap<u32, FactionStanding>,
    selection_guid: Option<ObjectGuid>,
    is_attacking: bool,
    attack_timers: [Duration; NUMBER_WEAPON_ATTACK_TYPES], // MainHand, OffHand, Ranged
}

impl Player {
    pub fn new() -> Self {
        Self {
            guid: None,
            name: "".to_owned(),
            values: InternalValues::new(PLAYER_END as usize),
            map_key: None,
            position: None,
            inventory: HashMap::new(),
            spells: Vec::new(),
            action_buttons: HashMap::new(),
            faction_standings: HashMap::new(),
            selection_guid: None,
            is_attacking: false,
            attack_timers: [
                Duration::from_millis(800), /* FIXME */
                Duration::ZERO,
                Duration::ZERO,
            ],
        }
    }

    pub fn create(
        conn: &mut PooledConnection<SqliteConnectionManager>,
        creation_payload: &CmsgCharCreate,
        account_id: u32,
        data_store: Arc<DataStore>,
    ) -> Result<(), Error> {
        let transaction = conn.transaction()?;

        let create_position: &PlayerCreatePosition = data_store
            .get_player_create_position(creation_payload.race as u32, creation_payload.class as u32)
            .expect("Missing player create position in DB");

        let character_guid = CharacterRepository::create_character(
            &transaction,
            creation_payload,
            account_id,
            create_position,
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
                    | InventoryType::WeaponMainHand => InventorySlot::EquipmentMainhand as u32,
                    InventoryType::Shield
                    | InventoryType::WeaponOffHand
                    | InventoryType::Quiver => InventorySlot::EquipmentOffhand as u32,
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

    pub fn load(
        &mut self,
        conn: &PooledConnection<SqliteConnectionManager>,
        account_id: u32,
        guid: u64,
        world_context: Arc<WorldContext>,
    ) {
        self.values.reset();

        let character = CharacterRepository::fetch_basic_character_data(conn, guid)
            .expect("Failed to load character from DB");

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

        let spells = CharacterRepository::fetch_character_spells(conn, guid);
        self.spells = spells;

        let guid = ObjectGuid::new(HighGuidType::Player, guid as u32);
        self.guid = Some(guid);
        self.values
            .set_u64(ObjectFields::ObjectFieldGuid.into(), self.guid().raw());

        self.name = character.name;

        let object_type = make_bitflags!(ObjectTypeMask::{Object | Unit | Player}).bits();
        self.values
            .set_u32(ObjectFields::ObjectFieldType.into(), object_type);

        self.values
            .set_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0);

        self.values
            .set_u32(UnitFields::UnitFieldLevel.into(), character.level as u32);
        self.values.set_u32(
            UnitFields::PlayerFieldMaxLevel.into(),
            world_context.config.world.game.player.maxlevel,
        );

        let race = CharacterRace::n(character.race).expect("Character has invalid race id in DB");
        self.values
            .set_u8(UnitFields::UnitFieldBytes0.into(), 0, race as u8);

        let class =
            CharacterClass::n(character.class).expect("Character has invalid class id in DB");
        self.values
            .set_u8(UnitFields::UnitFieldBytes0.into(), 1, class as u8);

        let gender = Gender::n(character.gender).expect("Character has invalid gender in DB");
        self.values
            .set_u8(UnitFields::UnitFieldBytes0.into(), 2, gender as u8);

        self.values
            .set_u8(UnitFields::UnitFieldBytes0.into(), 3, power_type as u8);

        self.values
            .set_u8(UnitFields::UnitFieldBytes2.into(), 1, 0x28); // UNIT_BYTE2_FLAG_UNK3 | UNIT_BYTE2_FLAG_UNK5

        self.position = Some(character.position);

        self.values.set_u8(
            UnitFields::PlayerBytes.into(),
            0,
            character.visual_features.skin,
        );
        self.values.set_u8(
            UnitFields::PlayerBytes.into(),
            1,
            character.visual_features.face,
        );
        self.values.set_u8(
            UnitFields::PlayerBytes.into(),
            2,
            character.visual_features.hairstyle,
        );
        self.values.set_u8(
            UnitFields::PlayerBytes.into(),
            3,
            character.visual_features.haircolor,
        );
        self.values.set_u8(
            UnitFields::PlayerBytes2.into(),
            0,
            character.visual_features.facialstyle,
        );
        self.values.set_u8(UnitFields::PlayerBytes2.into(), 3, 0x02); // Unk
        self.values
            .set_u8(UnitFields::PlayerBytes3.into(), 0, gender as u8);

        self.values
            .set_u32(UnitFields::UnitFieldDisplayid.into(), display_id);
        self.values
            .set_u32(UnitFields::UnitFieldNativedisplayid.into(), display_id);

        /* BEGIN TO REFACTOR LATER */
        self.values.set_u32(UnitFields::UnitFieldHealth.into(), 100);
        self.values
            .set_u32(UnitFields::UnitFieldMaxhealth.into(), 100);

        self.values.set_u32(
            UnitFields::UnitFieldPower1 as usize + power_type as usize,
            100,
        );
        self.values.set_u32(
            UnitFields::UnitFieldMaxpower1 as usize + power_type as usize,
            100,
        );

        self.values.set_u32(
            UnitFields::UnitFieldFactiontemplate.into(),
            chr_races_record.faction_id,
        );

        self.values
            .set_i32(UnitFields::PlayerFieldWatchedFactionIndex.into(), -1);

        // Skills
        let skills = CharacterRepository::fetch_character_skills(conn, guid.raw());
        for (index, skill) in skills.iter().enumerate() {
            self.values.set_u16(
                UnitFields::PlayerSkillInfo1_1 as usize + (index * 3),
                0,
                skill.skill_id,
            );
            // Note: PlayerSkillInfo1_1 offset 1 is "step"
            self.values.set_u16(
                UnitFields::PlayerSkillInfo1_1 as usize + 1 + (index * 3),
                0,
                skill.value,
            );
            self.values.set_u16(
                UnitFields::PlayerSkillInfo1_1 as usize + 1 + (index * 3),
                1,
                skill.max_value,
            );
        }

        self.values.set_u32(
            UnitFields::UnitFieldFlags.into(),
            UnitFlags::PlayerControlled as u32,
        );

        // Action buttons
        let action_buttons: HashMap<usize, ActionButton> =
            CharacterRepository::fetch_action_buttons(conn, guid.raw())
                .into_iter()
                .map(|button| (button.position as usize, button))
                .collect();
        self.action_buttons = action_buttons;

        // Reputations
        let faction_standings: HashMap<u32, FactionStanding> = {
            let records = CharacterRepository::fetch_faction_standings(conn, guid.raw());
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
                                        self.race().into(),
                                        self.class().into(),
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
        self.faction_standings = faction_standings;

        let inventory: HashMap<u32, Item> =
            ItemRepository::load_player_inventory(&conn, self.guid().raw() as u32)
                .into_iter()
                .map(|record| {
                    let item = Item::new(
                        record.guid,
                        record.entry,
                        record.owner_guid.unwrap(),
                        record.stack_count,
                    );
                    self.values.set_u64(
                        UnitFields::PlayerFieldInvSlotHead as usize + (2 * record.slot) as usize,
                        item.guid().raw(),
                    );

                    // Visible bits
                    if record.slot >= InventorySlot::EQUIPMENT_START
                        && record.slot < InventorySlot::EQUIPMENT_END
                    {
                        self.values.set_u32(
                            UnitFields::PlayerVisibleItem1_0 as usize
                                + (record.slot * MAX_PLAYER_VISIBLE_ITEM_OFFSET) as usize,
                            item.entry(),
                        );
                    }

                    (record.slot, item)
                })
                .collect();

        self.inventory = inventory;

        self.values.reset_dirty();
    }

    pub fn save(&mut self, transaction: &Transaction) -> Result<(), Error> {
        let mut stmt = transaction.prepare_cached("UPDATE characters SET map_id = :map_id, zone_id = :zone_id, position_x = :x, position_y = :y, position_z = :z, orientation = :o WHERE guid = :guid").unwrap();

        let position = self
            .position
            .as_ref()
            .expect("player has no position in Player::save");
        stmt.execute(named_params! {
            ":map_id": position.map,
            ":zone_id": position.zone,
            ":x": position.x,
            ":y": position.y,
            ":z": position.z,
            ":o": position.o,
            ":guid": self.guid().raw() as u32,
        })?;

        Ok(())
    }

    pub fn guid(&self) -> &ObjectGuid {
        self.guid
            .as_ref()
            .expect("Player guid uninitialized. Is the player in world?")
    }

    pub fn current_map(&self) -> Option<MapKey> {
        self.map_key
    }

    pub fn set_map(&mut self, map_key: MapKey) {
        self.map_key.replace(map_key);
    }

    pub fn race(&self) -> CharacterRace {
        let race_id = self.values.get_u8(UnitFields::UnitFieldBytes0.into(), 0);
        CharacterRace::n(race_id)
            .to_owned()
            .expect("Player race uninitialized. Is the player in world?")
    }

    pub fn class(&self) -> CharacterClass {
        let class_id = self.values.get_u8(UnitFields::UnitFieldBytes0.into(), 1);
        CharacterClass::n(class_id)
            .to_owned()
            .expect("Player class uninitialized. Is the player in world?")
    }

    pub fn level(&self) -> u8 {
        self.values.get_u32(UnitFields::UnitFieldLevel.into()) as u8
    }

    pub fn gender(&self) -> Gender {
        let gender_id = self.values.get_u8(UnitFields::UnitFieldBytes0.into(), 2);
        Gender::n(gender_id)
            .to_owned()
            .expect("Player gender uninitialized. Is the player in world?")
    }

    pub fn position(&self) -> &WorldPosition {
        self.position
            .as_ref()
            .expect("Player position uninitialized. Is the player in world?")
    }

    pub fn visual_features(&self) -> PlayerVisualFeatures {
        PlayerVisualFeatures {
            haircolor: self.values.get_u8(UnitFields::PlayerBytes.into(), 3),
            hairstyle: self.values.get_u8(UnitFields::PlayerBytes.into(), 2),
            face: self.values.get_u8(UnitFields::PlayerBytes.into(), 1),
            skin: self.values.get_u8(UnitFields::PlayerBytes.into(), 0),
            facialstyle: self.values.get_u8(UnitFields::PlayerBytes2.into(), 0),
        }
    }

    pub fn display_id(&self) -> u32 {
        self.values.get_u32(UnitFields::UnitFieldDisplayid.into())
    }

    pub fn native_display_id(&self) -> u32 {
        self.values
            .get_u32(UnitFields::UnitFieldNativedisplayid.into())
    }

    pub fn power_type(&self) -> PowerType {
        let power_type_id = self.values.get_u8(UnitFields::UnitFieldBytes0.into(), 3);
        PowerType::n(power_type_id)
            .to_owned()
            .expect("Player power type uninitialized. Is the player in world?")
    }

    pub fn set_position(&mut self, position: &Position) {
        let mut current_pos: WorldPosition = self
            .position
            .take()
            .expect("player has no world position in Player::set_position");

        current_pos.x = position.x;
        current_pos.y = position.y;
        current_pos.z = position.z;
        current_pos.o = position.o;

        self.position = Some(current_pos);
    }

    pub fn set_stand_state(&mut self, animstate: u32) {
        if UnitStandState::n(animstate).is_some() {
            self.values
                .set_u8(UnitFields::UnitFieldBytes1.into(), 0, animstate as u8);
        } else {
            warn!(
                "attempted to set an invalid stand state ({}) on player",
                animstate
            );
        }
    }

    pub fn set_sheath_state(&mut self, sheath_state: u32) {
        if SheathState::n(sheath_state).is_some() {
            // TODO: See Player::SetVirtualItemSlot in MaNGOS (enchantment visual stuff)
            self.values
                .set_u8(UnitFields::UnitFieldBytes2.into(), 0, sheath_state as u8);
        } else {
            warn!(
                "attempted to set an invalid sheath state ({}) on player",
                sheath_state
            );
        }
    }

    pub fn spells(&self) -> &Vec<u32> {
        &self.spells
    }

    pub fn action_buttons(&self) -> &HashMap<usize, ActionButton> {
        &self.action_buttons
    }

    pub fn faction_standings(&self) -> &HashMap<u32, FactionStanding> {
        &self.faction_standings
    }

    pub fn set_selection(&mut self, raw_guid: u64) {
        self.selection_guid = ObjectGuid::from_raw(raw_guid).filter(|&g| g != ObjectGuid::zero());
        self.values
            .set_u64(UnitFields::UnitFieldTarget.into(), raw_guid);
    }

    pub fn selection(&self) -> Option<ObjectGuid> {
        self.selection_guid
    }

    pub fn set_attacking(&mut self, is_attacking: bool) {
        self.is_attacking = is_attacking;
    }

    fn gen_create_data(&self) -> UpdateBlock {
        let mut update_builder = UpdateBlockBuilder::new();

        for index in 0..PLAYER_END {
            let value = self.values.get_u32(index as usize);
            if value != 0 {
                update_builder.add(index as usize, value);
            }
        }

        update_builder.build()
    }

    fn gen_update_data(&self) -> UpdateBlock {
        let mut update_builder = UpdateBlockBuilder::new();

        for index in self.values.get_dirty_indexes() {
            let value = self.values.get_u32(index as usize);
            update_builder.add(index as usize, value);
        }

        update_builder.build()
    }
}

pub struct PlayerVisualFeatures {
    pub haircolor: u8,
    pub hairstyle: u8,
    pub face: u8,
    pub skin: u8,
    pub facialstyle: u8,
}

#[async_trait]
impl WorldEntity for Player {
    fn guid(&self) -> &ObjectGuid {
        self.guid()
    }

    fn name(&self) -> String {
        self.name.to_owned()
    }

    async fn tick(&mut self, diff: Duration, world_context: Arc<WorldContext>) {
        for timer in self.attack_timers.iter_mut() {
            if *timer > diff {
                *timer -= diff;
            } else {
                *timer = Duration::ZERO;
            }
        }

        if let Some(selection_guid) = self.selection_guid {
            if self.attack_timers[WeaponAttackType::MainHand as usize].is_zero()
                && self.is_attacking
            {
                if let Some(target) = world_context
                    .map_manager
                    .lookup_entity(&selection_guid, self.map_key)
                    .await
                {
                    target.write().await.modify_health(-10);
                }

                let packet = ServerMessage::new(SmsgAttackerStateUpdate {
                    hit_info: 2, // TODO enum HitInfo
                    attacker_guid: self.guid().as_packed(),
                    target_guid: selection_guid.as_packed(),
                    actual_damage: 10,
                    sub_damage_count: 1,
                    sub_damage_school_mask: 1, // Physical
                    sub_damage: 10.0,
                    sub_damage_rounded: 10,
                    sub_damage_absorb: 0,
                    sub_damage_resist: 0,
                    target_state: 1, // TODO: Enum VictimState
                    unk1: 0,
                    spell_id: 0,
                    damage_blocked_amount: 0,
                });

                world_context
                    .map_manager
                    .broadcast_packet(self.guid(), self.map_key, &packet, None, true)
                    .await;

                self.attack_timers[WeaponAttackType::MainHand as usize] =
                    Duration::from_millis(800);
            }
        }
    }

    fn get_create_data(
        &self,
        recipient_guid: u64, // TODO: Change this to ObjectGuid
        world_context: Arc<WorldContext>,
    ) -> Vec<CreateData> {
        let movement = Some(MovementUpdateData {
            movement_flags: 0,  // 0x02000000, // TEMP: Flying
            movement_flags2: 0, // Always 0 in 2.4.3
            timestamp: world_context.game_time().as_millis() as u32, // Will overflow every 49.7 days
            position: Position {
                // FIXME: Into impl?
                x: self.position().x,
                y: self.position().y,
                z: self.position().z,
                o: self.position().o,
            },
            // pitch: Some(0.0),
            pitch: None,
            fall_time: 0,
            speed_walk: 2.5,
            speed_run: 7.0,
            speed_run_backward: 4.5,
            speed_swim: 4.722222,
            speed_swim_backward: 2.5,
            speed_flight: 70.0,
            speed_flight_backward: 4.5,
            speed_turn: 3.141594,
        });

        let flags = if recipient_guid == self.guid().raw() {
            make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition | SelfUpdate})
        } else {
            make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition})
        };

        let mut player_update_data = vec![CreateData {
            update_type: UpdateType::CreateObject2,
            packed_guid: self.guid().as_packed(),
            object_type: ObjectTypeId::Player,
            flags,
            movement,
            low_guid_part: None,
            high_guid_part: Some(HighGuidType::Player as u32),
            blocks: self.gen_create_data(),
        }];

        let inventory_updates: Vec<CreateData> = if recipient_guid == self.guid().raw() {
            self.inventory
                .iter()
                .flat_map(|item| {
                    item.1
                        .get_create_data(self.guid().raw(), world_context.clone())
                })
                .collect()
        } else {
            Vec::new()
        };

        player_update_data.extend(inventory_updates);
        player_update_data
    }

    fn get_update_data(
        &self,
        _recipient_guid: u64,
        _world_context: Arc<WorldContext>,
    ) -> Vec<UpdateData> {
        vec![UpdateData {
            update_type: UpdateType::Values,
            packed_guid: self.guid().as_packed(),
            blocks: self.gen_update_data(),
        }]
    }

    fn has_updates(&self) -> bool {
        self.values.has_dirty()
    }

    fn mark_up_to_date(&mut self) {
        self.values.reset_dirty();
    }

    fn modify_health(&mut self, damage: i32) {
        let current_health = self.values.get_i32(UnitFields::UnitFieldHealth.into());
        let max_health = self.values.get_i32(UnitFields::UnitFieldMaxhealth.into());
        let new_health = (current_health + damage).clamp(0, max_health) as u32;

        self.values
            .set_u32(UnitFields::UnitFieldHealth.into(), new_health);
    }
}
