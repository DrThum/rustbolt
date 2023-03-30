use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::{
    datastore::DataStore,
    repositories::character::CharacterRepository,
    shared::constants::{CharacterClass, CharacterRace, Gender, PowerType},
};

use super::{
    update::{UpdateData, UpdateDataBuilder},
    update_fields::*,
};

pub struct Player {
    guid: Option<u64>,
    race: Option<CharacterRace>,
    class: Option<CharacterClass>,
    level: Option<u8>,
    gender: Option<Gender>,
    visual_features: Option<PlayerVisualFeatures>,
    display_id: Option<u32>,
    native_display_id: Option<u32>,
    power_type: Option<PowerType>,
}

impl Player {
    pub fn new() -> Self {
        Self {
            guid: None,
            race: None,
            class: None,
            level: None,
            gender: None,
            visual_features: None,
            display_id: None,
            native_display_id: None,
            power_type: None,
        }
    }

    pub fn load(
        &mut self,
        conn: &mut PooledConnection<SqliteConnectionManager>,
        account_id: u32,
        guid: u64,
        data_store: &DataStore,
    ) {
        let character = CharacterRepository::fetch_basic_character_data(conn, guid, account_id)
            .expect("Failed to load character from DB");

        let chr_races_record = data_store
            .get_race_record(character.race as u32)
            .expect("Cannot load character because it has an invalid race id in DB");

        let display_id = if character.gender == Gender::Male as u8 {
            chr_races_record.male_display_id
        } else {
            chr_races_record.female_display_id
        };

        let power_type = data_store
            .get_class_record(character.class as u32)
            .map(|cl| PowerType::n(cl.power_type).unwrap())
            .expect("Cannot load character because it has an invalid class id in DB");

        self.guid = Some(guid);
        self.race =
            Some(CharacterRace::n(character.race).expect("Character has invalid race id in DB"));
        self.class =
            Some(CharacterClass::n(character.class).expect("Character has invalid class id in DB"));
        self.level = Some(character.level);
        self.gender =
            Some(Gender::n(character.gender).expect("Character has invalid gender in DB"));
        self.visual_features = Some(character.visual_features);
        self.display_id = Some(display_id);
        self.native_display_id = Some(display_id);
        self.power_type = Some(power_type);
    }

    pub fn guid(&self) -> &u64 {
        self.guid
            .as_ref()
            .expect("Player guid uninitialized. Is the player in world?")
    }

    pub fn race(&self) -> &CharacterRace {
        self.race
            .as_ref()
            .expect("Player race uninitialized. Is the player in world?")
    }

    pub fn class(&self) -> &CharacterClass {
        self.class
            .as_ref()
            .expect("Player class uninitialized. Is the player in world?")
    }

    pub fn level(&self) -> &u8 {
        self.level
            .as_ref()
            .expect("Player level uninitialized. Is the player in world?")
    }

    pub fn gender(&self) -> &Gender {
        self.gender
            .as_ref()
            .expect("Player gender uninitialized. Is the player in world?")
    }

    pub fn visual_features(&self) -> &PlayerVisualFeatures {
        self.visual_features
            .as_ref()
            .expect("Player visual features uninitialized. Is the player in world?")
    }

    pub fn display_id(&self) -> &u32 {
        self.display_id
            .as_ref()
            .expect("Player display id uninitialized. Is the player in world?")
    }

    pub fn native_display_id(&self) -> &u32 {
        self.native_display_id
            .as_ref()
            .expect("Player native display id uninitialized. Is the player in world?")
    }

    pub fn power_type(&self) -> &PowerType {
        self.power_type
            .as_ref()
            .expect("Player power type uninitialized. Is the player in world?")
    }

    pub fn gen_update_data(&self) -> UpdateData {
        let mut update_data_builder = UpdateDataBuilder::new();
        let visual_features = self.visual_features();

        update_data_builder.add_u64(ObjectFields::ObjectFieldGuid.into(), *self.guid());
        update_data_builder.add_u32(ObjectFields::ObjectFieldType.into(), 25);
        update_data_builder.add_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0);
        update_data_builder.add_u32(UnitFields::UnitFieldHealth.into(), 100);
        update_data_builder.add_u32(UnitFields::UnitFieldMaxhealth.into(), 100);
        update_data_builder.add_u32(
            UnitFields::UnitFieldPower1 as usize + *self.power_type() as usize,
            100,
        );
        update_data_builder.add_u32(
            UnitFields::UnitFieldMaxpower1 as usize + *self.power_type() as usize,
            100,
        );
        update_data_builder.add_u32(UnitFields::UnitFieldLevel.into(), *self.level() as u32);
        update_data_builder.add_u32(UnitFields::UnitFieldFactiontemplate.into(), 469); // FIXME
        update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 0, *self.race() as u8);
        update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 1, *self.class() as u8);
        update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 2, *self.gender() as u8);
        update_data_builder.add_u8(
            UnitFields::UnitFieldBytes0.into(),
            3,
            *self.power_type() as u8,
        );
        update_data_builder.add_u8(UnitFields::PlayerBytes.into(), 0, visual_features.skin);
        update_data_builder.add_u8(UnitFields::PlayerBytes.into(), 1, visual_features.face);
        update_data_builder.add_u8(UnitFields::PlayerBytes.into(), 2, visual_features.hairstyle);
        update_data_builder.add_u8(UnitFields::PlayerBytes.into(), 3, visual_features.haircolor);
        update_data_builder.add_u8(
            UnitFields::PlayerBytes2.into(),
            0,
            visual_features.facialstyle,
        );
        update_data_builder.add_u8(UnitFields::PlayerBytes2.into(), 3, 0x02); // Unk
        update_data_builder.add_u8(UnitFields::PlayerBytes3.into(), 0, *self.gender() as u8);
        update_data_builder.add_u32(UnitFields::UnitFieldDisplayid.into(), *self.display_id());
        update_data_builder.add_u32(
            UnitFields::UnitFieldNativedisplayid.into(),
            *self.native_display_id(),
        );

        update_data_builder.build()
    }
}

pub struct PlayerVisualFeatures {
    pub haircolor: u8,
    pub hairstyle: u8,
    pub face: u8,
    pub skin: u8,
    pub facialstyle: u8,
}
