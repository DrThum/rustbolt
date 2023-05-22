use std::hash::Hasher;

use binrw::binwrite;
use fixedbitset::FixedBitSet;

use crate::shared::constants::HighGuidType;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
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

    pub fn from_raw(raw: u64) -> Option<ObjectGuid> {
        let high_part = (raw >> 32) & 0xFFFFFFFF;
        HighGuidType::n(high_part as i64).map(|high_guid_type| ObjectGuid {
            high_guid_type,
            raw,
        })
    }

    pub fn with_entry(high_guid_type: HighGuidType, entry: u32, counter: u32) -> ObjectGuid {
        assert!(
            Self::has_entry_part(high_guid_type),
            "Attempt to create an ObjectGuid with an entry for a HighGuidType with no entry part"
        );

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
        if Self::has_entry_part(self.high_guid_type) {
            Some(((self.raw >> 24) & 0xFFFFFF) as u32)
        } else {
            None
        }
    }

    pub fn counter(&self) -> u32 {
        // Counter is 6 bytes if Guid has entry, 8 bytes otherwise
        if Self::has_entry_part(self.high_guid_type) {
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

    fn has_entry_part(high_guid_type: HighGuidType) -> bool {
        match high_guid_type {
            HighGuidType::ItemOrContainer
            | HighGuidType::Player
            | HighGuidType::Dynamicobject
            | HighGuidType::Corpse
            | HighGuidType::MoTransport
            | HighGuidType::Group => false,
            HighGuidType::Transport
            | HighGuidType::Unit
            | HighGuidType::Pet
            | HighGuidType::Gameobject => true,
        }
    }
}

impl std::hash::Hash for ObjectGuid {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

// Reduce the amount of bytes needed to transmit a guid by not sending zero-bytes in the full
// guid.
//
// - `mask` indicates the bytes that will be transmitted: each of the bits in the mask represent a
//   byte in the full guid
// - `bytes` contains the bytes to transmit from least to most significant
#[binwrite]
#[derive(Debug, Clone)]
pub struct PackedObjectGuid {
    mask: u8,
    bytes: Vec<u8>,
}
