use std::sync::Arc;

use fixedbitset::FixedBitSet;
use parking_lot::RwLock;
use shipyard::Component;

use super::object_guid::ObjectGuid;

pub struct InternalValues {
    size: usize,
    values: Vec<RwLock<Value>>,
    dirty_indexes: RwLock<FixedBitSet>,
}

impl InternalValues {
    pub fn new(size: usize) -> InternalValues {
        let mut values = Vec::new();
        values.resize(size, Value { as_u32: 0 });
        let values = values.into_iter().map(|v| RwLock::new(v)).collect();

        InternalValues {
            size,
            values,
            dirty_indexes: RwLock::new(FixedBitSet::with_capacity(size)),
        }
    }

    fn mark_dirty(&self, index: usize) {
        self.dirty_indexes.write().set(index, true);
    }

    pub fn has_dirty(&self) -> bool {
        !self.dirty_indexes.read().is_clear()
    }

    pub fn get_dirty_indexes(&self) -> Vec<usize> {
        self.dirty_indexes.read().ones().collect()
    }

    pub fn reset_dirty(&self) {
        self.dirty_indexes.write().clear();
    }

    pub fn set_u32(&self, index: usize, value: u32) {
        assert!(index < self.size, "index is too high"); // TODO: remove these asserts about size?
                                                         // (keep the ones about the offset)

        self.mark_dirty(index);
        *self.values[index].write() = Value { as_u32: value };
    }

    pub fn get_u32(&self, index: usize) -> u32 {
        assert!(index < self.size, "index is too high");

        unsafe { self.values[index].read().as_u32 }
    }

    pub fn set_u8(&self, index: usize, offset: usize, value: u8) {
        assert!(index < self.size, "index is too high");
        assert!(offset < 4, "offset is too high");

        let existing_as_u32 = self.get_u32(index);
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

    pub fn get_u8(&self, index: usize, offset: u8) -> u8 {
        assert!(index < self.size, "index is too high");
        assert!(offset < 4, "offset is too high");

        unsafe { ((self.values[index].read().as_u32 >> (offset * 8)) & 0xFF) as u8 }
    }

    #[allow(dead_code)]
    pub fn set_u16(&self, index: usize, offset: u8, value: u16) {
        assert!(index < self.size, "index is too high");
        assert!(offset < 2, "offset is too high");

        let existing_as_u32 = self.get_u32(index);
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

    #[allow(dead_code)]
    pub fn get_u16(&self, index: usize, offset: u8) -> u16 {
        assert!(index < self.size, "index is too high");
        assert!(offset < 2, "offset is too high");

        unsafe { ((self.values[index].read().as_u32 >> (offset * 16)) & 0xFFFF) as u16 }
    }

    pub fn set_u64(&self, index: usize, value: u64) {
        assert!(index < (self.size - 1), "index is too high");

        self.set_u32(index, (value & 0xFFFFFFFF) as u32);
        self.set_u32(index + 1, ((value >> 32) & 0xFFFFFFFF) as u32);
    }

    #[allow(dead_code)]
    pub fn get_u64(&self, index: usize) -> u64 {
        assert!(index < (self.size - 1), "index is too high");

        self.get_u32(index) as u64 | (self.get_u32(index + 1) as u64) << 32
    }

    #[allow(dead_code)]
    pub fn set_guid(&self, index: usize, value: &ObjectGuid) {
        self.set_u64(index, value.raw());
    }

    pub fn set_f32(&self, index: usize, value: f32) {
        assert!(index < self.size, "index is too high");

        self.mark_dirty(index);
        *self.values[index].write() = Value { as_f32: value };
    }

    #[allow(dead_code)]
    pub fn get_f32(&self, index: usize) -> f32 {
        assert!(index < self.size, "index is too high");

        unsafe { self.values[index].read().as_f32 }
    }

    pub fn set_i32(&self, index: usize, value: i32) {
        assert!(index < self.size, "index is too high");

        self.mark_dirty(index);
        *self.values[index].write() = Value { as_i32: value };
    }

    #[allow(dead_code)]
    pub fn get_i32(&self, index: usize) -> i32 {
        assert!(index < self.size, "index is too high");

        unsafe { self.values[index].read().as_i32 }
    }

    #[allow(dead_code)]
    pub fn set_flag_u32(&self, index: usize, flag: u32) {
        assert!(index < self.size, "index is too high");

        let current = self.get_u32(index);
        let new = current | flag;

        if current != new {
            self.set_u32(index, new);
        }
    }

    #[allow(dead_code)]
    pub fn unset_flag_u32(&self, index: usize, flag: u32) {
        assert!(index < self.size, "index is too high");

        let current = self.get_u32(index);
        let new = current & !flag;

        if current != new {
            self.set_u32(index, new);
        }
    }

    #[allow(dead_code)]
    pub fn has_flag_u32(&self, index: usize, flag: u32) -> bool {
        assert!(index < self.size, "index is too high");

        self.get_u32(index) & flag != 0
    }
}

#[derive(Component)]
pub struct WrappedInternalValues(pub Arc<InternalValues>);

#[derive(Clone, Copy)]
pub union Value {
    pub as_u32: u32,
    pub as_f32: f32,
    pub as_i32: i32,
}

pub const QUEST_SLOT_OFFSETS_COUNT: usize = 4;

#[allow(dead_code)]
pub enum QuestSlotOffset {
    Entry = 0,
    State = 1,
    Counters = 2,
    Timer = 3,
}
