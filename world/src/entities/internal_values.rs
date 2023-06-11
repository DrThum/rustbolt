use std::sync::Arc;

use enumflags2::make_bitflags;
use fixedbitset::{FixedBitSet, Ones};
use parking_lot::RwLock;
use rand::{seq::SliceRandom, Rng};
use shipyard::Component;

use crate::{
    game::world_context::WorldContext,
    repositories::{
        character::CharacterRepository, creature::CreatureSpawnDbRecord, item::ItemRepository,
    },
    shared::constants::{
        CharacterClass, CharacterRace, Gender, InventorySlot, ObjectTypeMask, PowerType, UnitFlags,
        PLAYER_DEFAULT_COMBAT_REACH,
    },
    DataStore,
};

use super::{
    item::Item,
    object_guid::ObjectGuid,
    update_fields::{
        ObjectFields, UnitFields, MAX_PLAYER_VISIBLE_ITEM_OFFSET, PLAYER_END, UNIT_END,
    },
};

pub struct InternalValues {
    size: usize,
    values: Vec<Value>,
    dirty_indexes: FixedBitSet,
}

impl InternalValues {
    pub fn new(size: usize) -> InternalValues {
        let mut values = Vec::new();
        values.resize(size, Value { as_u32: 0 });

        InternalValues {
            size,
            values,
            dirty_indexes: FixedBitSet::with_capacity(size),
        }
    }

    pub fn reset(&mut self) {
        self.values.clear();
        self.values.resize(self.size, Value { as_u32: 0 });
    }

    fn mark_dirty(&mut self, index: usize) {
        self.dirty_indexes.set(index, true);
    }

    pub fn has_dirty(&self) -> bool {
        !self.dirty_indexes.is_clear()
    }

    pub fn get_dirty_indexes(&self) -> Ones {
        self.dirty_indexes.ones()
    }

    pub fn reset_dirty(&mut self) {
        self.dirty_indexes.clear();
    }

    pub fn set_u32(&mut self, index: usize, value: u32) {
        assert!(index < self.size, "index is too high");

        self.mark_dirty(index);
        self.values[index] = Value { as_u32: value };
    }

    pub fn get_u32(&self, index: usize) -> u32 {
        assert!(index < self.size, "index is too high");

        unsafe { self.values[index].as_u32 }
    }

    pub fn set_u8(&mut self, index: usize, offset: u8, value: u8) {
        assert!(index < self.size, "index is too high");
        assert!(offset < 4, "offset is too high");

        unsafe {
            let existing_as_u32 = self.values[index].as_u32;
            let reset_mask: u32 = match offset {
                // Reset relevant bytes to zero first...
                0 => 0xFFFFFF00,
                1 => 0xFFFF00FF,
                2 => 0xFF00FFFF,
                3 => 0x00FFFFFF,
                _ => 0xFFFFFFFF,
            };

            let updated_as_u32 = existing_as_u32 & reset_mask;
            // ... Then, set them to the new value
            let updated_as_u32 = updated_as_u32 | ((value as u32) << (offset * 8));
            self.set_u32(index, updated_as_u32);
        }
    }

    pub fn get_u8(&self, index: usize, offset: u8) -> u8 {
        assert!(index < self.size, "index is too high");
        assert!(offset < 4, "offset is too high");

        unsafe { ((self.values[index].as_u32 >> (offset * 8)) & 0xFF) as u8 }
    }

    #[allow(dead_code)]
    pub fn set_u16(&mut self, index: usize, offset: u8, value: u16) {
        assert!(index < self.size, "index is too high");
        assert!(offset < 2, "offset is too high");

        unsafe {
            let existing_as_u32 = self.values[index].as_u32;
            let reset_mask: u32 = match offset {
                // Reset relevant bytes to zero first...
                0 => 0xFFFF0000,
                1 => 0x0000FFFF,
                _ => 0xFFFFFFFF,
            };

            let updated_as_u32 = existing_as_u32 & reset_mask;
            // ... Then, set them to the new value
            let updated_as_u32 = updated_as_u32 | ((value as u32) << (offset * 16));
            self.set_u32(index, updated_as_u32);
        }
    }

    #[allow(dead_code)]
    pub fn get_u16(&self, index: usize, offset: u8) -> u16 {
        assert!(index < self.size, "index is too high");
        assert!(offset < 2, "offset is too high");

        unsafe { ((self.values[index].as_u32 >> (offset * 16)) & 0xFFFF) as u16 }
    }

    pub fn set_u64(&mut self, index: usize, value: u64) {
        assert!(index < (self.size - 1), "index is too high");

        self.set_u32(index, (value & 0xFFFFFFFF) as u32);
        self.set_u32(index + 1, ((value >> 32) & 0xFFFFFFFF) as u32);
    }

    #[allow(dead_code)]
    pub fn get_u64(&mut self, index: usize) -> u64 {
        assert!(index < (self.size - 1), "index is too high");

        self.get_u32(index) as u64 | (self.get_u32(index + 1) as u64) << 32
    }

    pub fn set_f32(&mut self, index: usize, value: f32) {
        assert!(index < self.size, "index is too high");

        self.mark_dirty(index);
        self.values[index] = Value { as_f32: value };
    }

    #[allow(dead_code)]
    pub fn get_f32(&self, index: usize) -> f32 {
        assert!(index < self.size, "index is too high");

        unsafe { self.values[index].as_f32 }
    }

    pub fn set_i32(&mut self, index: usize, value: i32) {
        assert!(index < self.size, "index is too high");

        self.mark_dirty(index);
        self.values[index] = Value { as_i32: value };
    }

    #[allow(dead_code)]
    pub fn get_i32(&self, index: usize) -> i32 {
        assert!(index < self.size, "index is too high");

        unsafe { self.values[index].as_i32 }
    }

    pub fn build_for_creature(
        creature_spawn: &CreatureSpawnDbRecord,
        data_store: Arc<DataStore>,
        guid: &ObjectGuid,
    ) -> Option<InternalValues> {
        data_store
            .get_creature_template(creature_spawn.entry)
            .map(|template| {
                let mut rng = rand::thread_rng();

                let mut values = InternalValues::new(UNIT_END as usize);
                values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());

                let object_type = make_bitflags!(ObjectTypeMask::{Object | Unit}).bits();
                values.set_u32(ObjectFields::ObjectFieldType.into(), object_type);

                values.set_u32(ObjectFields::ObjectFieldEntry.into(), template.entry);

                values.set_f32(ObjectFields::ObjectFieldScaleX.into(), template.scale);

                values.set_u32(
                    UnitFields::UnitFieldLevel.into(),
                    rng.gen_range(template.min_level..=template.max_level),
                );

                let existing_model_ids: Vec<&u32> =
                    template.model_ids.iter().filter(|&&id| id != 0).collect();
                let display_id = existing_model_ids.choose(&mut rng).expect("rng error");
                values.set_u32(UnitFields::UnitFieldDisplayid.into(), **display_id);
                values.set_u32(UnitFields::UnitFieldNativedisplayid.into(), **display_id);
                // TODO: CombatReach must come from a DBC
                values.set_f32(UnitFields::UnitFieldCombatReach.into(), 1.5);

                values.set_u32(UnitFields::UnitFieldHealth.into(), 100); // TODO
                values.set_u32(UnitFields::UnitFieldMaxhealth.into(), 100); // TODO

                values.set_u32(
                    UnitFields::UnitFieldFactiontemplate.into(),
                    template.faction_template_id,
                );

                values.set_u32(UnitFields::UnitNpcFlags.into(), template.npc_flags);
                values.set_u32(UnitFields::UnitFieldFlags.into(), template.unit_flags);
                values.set_u32(UnitFields::UnitDynamicFlags.into(), template.dynamic_flags);

                values
            })
    }

    pub fn build_for_player(guid: &ObjectGuid, world_context: Arc<WorldContext>) -> InternalValues {
        let conn = world_context.database.characters.get().unwrap();
        let character = CharacterRepository::fetch_basic_character_data(&conn, guid.raw())
            .expect("Failed to load character from DB");

        let mut values = InternalValues::new(PLAYER_END as usize);

        values.set_u64(ObjectFields::ObjectFieldGuid.into(), guid.raw());

        let object_type = make_bitflags!(ObjectTypeMask::{Object | Unit | Player}).bits();
        values.set_u32(ObjectFields::ObjectFieldType.into(), object_type);

        values.set_f32(ObjectFields::ObjectFieldScaleX.into(), 1.0);

        values.set_u32(UnitFields::UnitFieldLevel.into(), character.level as u32);
        values.set_u32(
            UnitFields::PlayerFieldMaxLevel.into(),
            world_context.config.world.game.player.maxlevel,
        );

        let race = CharacterRace::n(character.race).expect("Character has invalid race id in DB");
        values.set_u8(UnitFields::UnitFieldBytes0.into(), 0, race as u8);

        let class =
            CharacterClass::n(character.class).expect("Character has invalid class id in DB");
        values.set_u8(UnitFields::UnitFieldBytes0.into(), 1, class as u8);

        let gender = Gender::n(character.gender).expect("Character has invalid gender in DB");
        values.set_u8(UnitFields::UnitFieldBytes0.into(), 2, gender as u8);

        let power_type = world_context
            .data_store
            .get_class_record(character.class as u32)
            .map(|cl| PowerType::n(cl.power_type).unwrap())
            .expect("Cannot load character because it has an invalid class id in DB");
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

        let chr_races_record = world_context
            .data_store
            .get_race_record(character.race as u32)
            .expect("Cannot load character because it has an invalid race id in DB");
        let display_id = if character.gender == Gender::Male as u8 {
            chr_races_record.male_display_id
        } else {
            chr_races_record.female_display_id
        };
        values.set_u32(UnitFields::UnitFieldDisplayid.into(), display_id);
        values.set_u32(UnitFields::UnitFieldNativedisplayid.into(), display_id);
        values.set_f32(
            UnitFields::UnitFieldCombatReach.into(),
            PLAYER_DEFAULT_COMBAT_REACH,
        );

        /* BEGIN TO REFACTOR LATER */
        values.set_u32(UnitFields::UnitFieldHealth.into(), 100);
        values.set_u32(UnitFields::UnitFieldMaxhealth.into(), 100);

        let power_type = world_context
            .data_store
            .get_class_record(character.class as u32)
            .map(|cl| PowerType::n(cl.power_type).unwrap())
            .expect("Cannot load character because it has an invalid class id in DB");
        values.set_u32(
            UnitFields::UnitFieldPower1 as usize + power_type as usize,
            100,
        );
        values.set_u32(
            UnitFields::UnitFieldMaxpower1 as usize + power_type as usize,
            100,
        );

        values.set_u32(
            UnitFields::UnitFieldFactiontemplate.into(),
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

        let _ = ItemRepository::load_player_inventory(&conn, guid.raw() as u32)
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
            });

        values.reset_dirty();

        values
    }
}

#[derive(Component)]
pub struct WrappedInternalValues(pub Arc<RwLock<InternalValues>>);

#[derive(Clone, Copy)]
pub union Value {
    pub as_u32: u32,
    pub as_f32: f32,
    pub as_i32: i32,
}
