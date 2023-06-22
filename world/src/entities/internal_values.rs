use std::sync::Arc;

use fixedbitset::{FixedBitSet, Ones};
use parking_lot::RwLock;
use shipyard::Component;

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
}

#[derive(Component)]
pub struct WrappedInternalValues(pub Arc<RwLock<InternalValues>>);

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
