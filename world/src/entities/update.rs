use fixedbitset::FixedBitSet;
use log::error;

// Represent a value to be updated on the client for an entity.
//
// index: check [update_fields] for possible values
// value: all values are sent as 4 bytes, with padding if needed
struct UpdateBlock {
    pub index: usize,
    pub value: [u8; 4],
}

impl UpdateBlock {
    pub fn empty(index: usize) -> UpdateBlock {
        UpdateBlock {
            index,
            value: [0_u8; 4],
        }
    }
}

pub struct UpdateDataBuilder {
    block_masks: Vec<FixedBitSet>,
    blocks: Vec<UpdateBlock>,
}

// Formatted update data for the client:
//
// * num_masks represent the number of block_masks to expect
// * block_masks contains bits whose index indicates which fields are being updated
// * data contains the data, one value for each bit set to 1 in the masks, in the same order
pub struct UpdateData {
    pub num_masks: u8,
    pub block_masks: Vec<u32>,
    pub data: Vec<[u8; 4]>,
}

impl UpdateDataBuilder {
    pub fn new() -> UpdateDataBuilder {
        UpdateDataBuilder {
            block_masks: vec![],
            blocks: vec![],
        }
    }

    pub fn add_u8(&mut self, index: usize, offset: u8, value: u8) {
        // Set the bit in the mask even if it's already set for this offset, and add this u8 to the
        // correct offset to the existing value with & (or 0 if it's not defined yet)
        if offset < 4 {
            let default_ub = UpdateBlock::empty(index);
            let update_block: &UpdateBlock = self
                .blocks
                .iter()
                .find(|ub| ub.index == index)
                .unwrap_or(&default_ub);

            let existing_as_u32 = u32::from_le_bytes(update_block.value);
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

            self.add_u32(index, updated_as_u32);
        } else {
            error!("UpdateDataBuilder: add_u8 expects an offset between 0 and 3 (inclusive)");
        }
    }

    pub fn add_u16(&mut self, index: usize, offset: u8, value: u16) {
        // Set the bit in the mask even if it's already set for this offset, and add this u8 to the
        // correct offset to the existing value with & (or 0 if it's not defined yet)
        if offset < 2 {
            let default_ub = UpdateBlock::empty(index);
            let update_block: &UpdateBlock = self
                .blocks
                .iter()
                .find(|ub| ub.index == index)
                .unwrap_or(&default_ub);

            let existing_as_u32 = u32::from_le_bytes(update_block.value);
            let reset_mask: u32 = match offset {
                // Reset relevant bytes to zero first...
                0 => 0xFFFF0000,
                1 => 0x0000FFFF,
                _ => 0xFFFFFFFF,
            };

            let updated_as_u32 = existing_as_u32 & reset_mask;
            // ... Then, set them to the new value
            let updated_as_u32 = updated_as_u32 | ((value as u32) << (offset * 16));

            self.add_u32(index, updated_as_u32);
        } else {
            error!("UpdateDataBuilder: add_u16 expects an offset between 0 and 1 (inclusive)");
        }
    }

    pub fn add_u32(&mut self, index: usize, value: u32) {
        let block_index = index / 32;
        let block_offset = index % 32;

        if block_index >= self.block_masks.len() {
            // Ensure that we have enough masks
            self.block_masks
                .resize(block_index + 1, FixedBitSet::with_capacity(32));
        }

        let block_bitset: &mut FixedBitSet = &mut self.block_masks[block_index];
        let was_already_set = block_bitset.put(block_offset);
        if was_already_set {
            // Remove the existing value first
            self.blocks.retain(|ub| ub.index != index);
        }

        self.blocks.push(UpdateBlock {
            index,
            value: value.to_le_bytes(),
        });
    }

    pub fn add_u64(&mut self, index: usize, value: u64) {
        // Add the lowest 32 bits at index and the highest 32 bits at index+1
        let high_bits: u32 = ((value & 0xFFFFFFFF00000000) >> 32) as u32;
        let low_bits: u32 = (value & 0x00000000FFFFFFFF) as u32;

        self.add_u32(index, low_bits);
        self.add_u32(index + 1, high_bits);
    }

    pub fn add_f32(&mut self, index: usize, value: f32) {
        // Interpret the bytes as u32
        let bytes = value.to_le_bytes();
        let as_u32 = u32::from_le_bytes(bytes);

        self.add_u32(index, as_u32);
    }

    pub fn build(mut self) -> UpdateData {
        let block_masks: Vec<u32> = self
            .block_masks
            .iter()
            .map(|mask| *mask.as_slice().first().unwrap())
            .collect();

        self.blocks.sort_by_key(|b| b.index);
        let data: Vec<[u8; 4]> = self.blocks.into_iter().map(|b| b.value).collect();

        UpdateData {
            num_masks: self.block_masks.len() as u8,
            block_masks,
            data,
        }
    }
}