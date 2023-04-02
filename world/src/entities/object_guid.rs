use binrw::binwrite;
use fixedbitset::FixedBitSet;

use crate::shared::constants::HighGuidType;

pub struct ObjectGuid {
    high_guid_type: HighGuidType,
    raw: u64,
}

impl ObjectGuid {
    pub fn new(high_guid_type: HighGuidType, counter: u32) -> ObjectGuid {
        let raw = ((high_guid_type as u64) << 48) | counter as u64;

        ObjectGuid {
            high_guid_type,
            raw,
        }
    }

    pub fn with_entry(high_guid_type: HighGuidType, entry: u32, counter: u32) -> ObjectGuid {
        // TODO: Validate that this HighGuidType has an entry part
        let raw = ((high_guid_type as u64) << 48) | ((entry as u64) << 24) | counter as u64;

        ObjectGuid {
            high_guid_type,
            raw,
        }
    }

    pub fn raw(&self) -> u64 {
        self.raw
    }

    pub fn high_part(&self) -> u64 {
        self.high_guid_type as u64
    }

    pub fn entry_part(&self) -> Option<u32> {
        match self.high_guid_type {
            HighGuidType::ItemOrContainer
            | HighGuidType::Player
            | HighGuidType::Dynamicobject
            | HighGuidType::Corpse
            | HighGuidType::MoTransport
            | HighGuidType::Group => None,
            HighGuidType::Transport
            | HighGuidType::Unit
            | HighGuidType::Pet
            | HighGuidType::Gameobject => Some(((self.raw >> 24) & 0xFFFFFF) as u32),
        }
    }

    pub fn counter(&self) -> u32 {
        // Counter is 6 bytes if Guid has entry, 8 bytes otherwise
        if let Some(_) = self.entry_part() {
            (self.raw & 0xFFFFFF) as u32
        } else {
            (self.raw & 0xFFFFFFFF) as u32
        }
    }

    pub fn as_packed(&self) -> PackedObjectGuid {
        let mut mask = FixedBitSet::with_capacity(8);
        let mut bytes: Vec<u8> = Vec::new();

        for i in 0..8 {
            let current_byte: u8 = ((self.raw & (0xFF << (i * 8))) >> (i * 8)) as u8;
            if current_byte != 0 {
                mask.set(i, true);
                bytes.push(current_byte);
            }
        }

        PackedObjectGuid {
            mask: *mask.as_slice().first().unwrap() as u8,
            bytes,
        }
    }
}

// Reduce the amount of bytes needed to transmit a guid by not sending zero-bytes in the full
// guid.
//
// - `mask` indicates the bytes that will be transmitted: each of the bits in the mask represent a
//   byte in the full guid
// - `bytes` contains the bytes to transmit from least to most significant
#[binwrite]
#[derive(Debug)]
pub struct PackedObjectGuid {
    mask: u8,
    bytes: Vec<u8>,
}
