use crate::shared::constants::{CharacterClass, CharacterRace, Gender};

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
        }
    }

    pub fn setup_entering_world(
        &mut self,
        guid: u64,
        race: CharacterRace,
        class: CharacterClass,
        level: u8,
        gender: Gender,
        visual_features: PlayerVisualFeatures,
        display_id: u32,
    ) {
        self.guid = Some(guid);
        self.race = Some(race);
        self.class = Some(class);
        self.level = Some(level);
        self.gender = Some(gender);
        self.visual_features = Some(visual_features);
        self.display_id = Some(display_id);
        self.native_display_id = Some(display_id);
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

    pub fn gen_update_data(&self) -> UpdateData {
        let mut update_data_builder = UpdateDataBuilder::new();
        let visual_features = self.visual_features();

        update_data_builder.add_u64(ObjectFields::ObjectFieldGuid.into(), *self.guid());
        update_data_builder.add_u32(ObjectFields::ObjectFieldType.into(), 25);
        update_data_builder.add_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0);
        update_data_builder.add_u32(UnitFields::UnitFieldHealth.into(), 100);
        update_data_builder.add_u32(UnitFields::UnitFieldMaxhealth.into(), 100);
        update_data_builder.add_u32(UnitFields::UnitFieldLevel.into(), *self.level() as u32);
        update_data_builder.add_u32(UnitFields::UnitFieldFactiontemplate.into(), 469); // FIXME
        update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 0, *self.race() as u8);
        update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 1, *self.class() as u8);
        update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 2, *self.gender() as u8);
        update_data_builder.add_u8(UnitFields::UnitFieldBytes0.into(), 3, 0); // powertype, 0 = MANA // TODO: ChrClasses.dbc
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
