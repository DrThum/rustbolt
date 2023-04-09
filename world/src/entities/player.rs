use std::{collections::HashMap, sync::Arc};

use enumflags2::make_bitflags;
use log::error;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Error;

use crate::{
    datastore::{data_types::PlayerCreatePosition, DataStore},
    protocol::packets::CmsgCharCreate,
    repositories::{character::CharacterRepository, item::ItemRepository},
    shared::constants::{
        CharacterClass, CharacterRace, Gender, HighGuidType, InventorySlot, InventoryType,
        ItemClass, ItemSubclassConsumable, ObjectTypeMask, PowerType,
    },
    world_context::WorldContext,
};

use super::{
    internal_values::InternalValues,
    item::Item,
    object_guid::ObjectGuid,
    update::{
        MovementUpdateData, UpdatableEntity, UpdateBlock, UpdateBlockBuilder, UpdateData,
        UpdateFlag, UpdateType,
    },
    update_fields::*,
    ObjectTypeId, Position, WorldPosition,
};

pub type PlayerInventory = HashMap<u32, Item>; // Key is slot

pub struct Player {
    guid: Option<ObjectGuid>,
    values: InternalValues,
    position: Option<WorldPosition>,
    inventory: PlayerInventory,
}

impl Player {
    pub fn new() -> Self {
        Self {
            guid: None,
            values: InternalValues::new(PLAYER_END as usize),
            position: None,
            inventory: HashMap::new(),
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

        let guid = ObjectGuid::new(HighGuidType::Player, guid as u32);
        self.guid = Some(guid);
        self.values
            .set_u64(ObjectFields::ObjectFieldGuid.into(), self.guid().raw());

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
            .set_u32(UnitFields::UnitFieldBytes2.into(), 0x28); // Unk

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

        self.values
            .set_u32(UnitFields::UnitFieldFactiontemplate.into(), 469);
        /* END TO REFACTOR LATER */

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
    }

    pub fn guid(&self) -> &ObjectGuid {
        self.guid
            .as_ref()
            .expect("Player guid uninitialized. Is the player in world?")
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
}

pub struct PlayerVisualFeatures {
    pub haircolor: u8,
    pub hairstyle: u8,
    pub face: u8,
    pub skin: u8,
    pub facialstyle: u8,
}

impl UpdatableEntity for Player {
    fn get_create_data(
        &self,
        recipient_guid: u64,
        world_context: Arc<WorldContext>,
    ) -> Vec<UpdateData> {
        let movement = Some(MovementUpdateData {
            movement_flags: 0,
            movement_flags2: 0, // Always 0 in 2.4.3
            timestamp: world_context.game_time().as_millis() as u32, // Will overflow every 49.7 days
            position: Position {
                // FIXME: Into impl?
                x: self.position().x,
                y: self.position().y,
                z: self.position().z,
                o: self.position().o,
            },
            fall_time: 0,
            speed_walk: 1.0,
            speed_run: 70.0,
            speed_run_backward: 4.5,
            speed_swim: 0.0,
            speed_swim_backward: 0.0,
            speed_flight: 70.0,
            speed_flight_backward: 4.5,
            speed_turn: 3.1415,
        });

        let flags = if recipient_guid == self.guid().raw() {
            make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition | SelfUpdate})
        } else {
            make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition})
        };

        let mut player_update_data = vec![UpdateData {
            update_type: UpdateType::CreateObject2,
            packed_guid: self.guid().as_packed(),
            object_type: ObjectTypeId::Player,
            flags,
            movement,
            low_guid_part: None,
            high_guid_part: Some(HighGuidType::Player as u32),
            blocks: self.gen_create_data(),
        }];

        let inventory_updates: Vec<UpdateData> = self
            .inventory
            .iter()
            .flat_map(|item| {
                item.1
                    .get_create_data(self.guid().raw(), world_context.clone())
            })
            .collect();

        player_update_data.extend(inventory_updates);
        player_update_data
    }

    fn get_update_data(
        &self,
        _recipient_guid: u64,
        _world_context: Arc<WorldContext>,
    ) -> Vec<UpdateData> {
        todo!();
    }
}
