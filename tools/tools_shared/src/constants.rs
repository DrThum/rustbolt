#[allow(dead_code)]
pub enum MPQFileFlags {
    // The file is compressed using PKWARE Data compression library
    CompressedWithPKWare = 0x100,
    // The file is compressed using a combination of compression methods
    CompressedMulti = 0x200,
    // The file is not compressed
    NotCompressed = 0x300,
    // The file is encrypted
    Encrypted = 0x10000,
    // The decryption key for the file is altered according to the position of the file in the archive
    FixKey = 0x20000,
    // The file contains incremental patch for an existing file in base MPQ
    PatchFile = 0x00100000,
    // Instead of being divided to 0x1000-bytes blocks, the file is stored as single unit
    SingleUnit = 0x01000000,
    // File is a deletion marker, indicating that the file no longer exists.
    // This is used to allow patch archives to delete files present in lower-priority archives in the search chain.
    // The file usually has length of 0 or 1 byte and its name is a hash
    DeleteMarker = 0x02000000,
    // File has checksums for each sector. Ignored if file is not compressed or compressed with
    // PKWare.
    SectorCrc = 0x04000000,
    // Set if file exists, reset when the file was deleted
    FileExists = 0x80000000,
}

pub const HASH_TABLE_ENTRY_SIZE: usize = 16;
pub const BLOCK_TABLE_ENTRY_SIZE: usize = 16;
pub const BLOCK_ENTRY_IS_FREE: u32 = 0xFFFFFFFF;

pub const HASH_TABLE_HASH_OFFSET: u32 = 0;
pub const HASH_TABLE_HASH_A_OFFSET: u32 = 1;
pub const HASH_TABLE_HASH_B_OFFSET: u32 = 2;
