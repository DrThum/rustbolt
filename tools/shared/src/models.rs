use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use binrw::{binread, io::Cursor, BinReaderExt};
use bytemuck::cast_slice;
use log::trace;

use crate::{
    constants::{
        BLOCK_TABLE_ENTRY_SIZE, HASH_TABLE_ENTRY_SIZE, HASH_TABLE_HASH_A_OFFSET,
        HASH_TABLE_HASH_B_OFFSET, HASH_TABLE_HASH_OFFSET,
    },
    utils::{compression::decompress, mpq::hash_string},
};

pub struct MPQFile {
    pub header: MPQHeader,
    pub hash_table: Vec<u8>,
    pub block_table: Vec<u8>,
    underlying_file: File,
}

impl MPQFile {
    pub fn new(
        header: MPQHeader,
        hash_table: Vec<u8>,
        block_table: Vec<u8>,
        file: File,
    ) -> MPQFile {
        MPQFile {
            header,
            hash_table,
            block_table,
            underlying_file: file,
        }
    }

    pub fn find_hash_table_entry(
        &self,
        file_name: &str,
        crypt_table: &[u32; 0x500],
    ) -> Option<MPQHashTableEntry> {
        fn get_entry_at(decrypted_hash_table: &Vec<u8>, file_number: u32) -> MPQHashTableEntry {
            let mut reader = Cursor::new(decrypted_hash_table);
            reader
                .seek(SeekFrom::Start(
                    file_number as u64 * HASH_TABLE_ENTRY_SIZE as u64,
                ))
                .unwrap();
            let entry: MPQHashTableEntry = reader.read_le().unwrap();
            entry
        }

        let hash: u32 = hash_string(crypt_table, file_name, HASH_TABLE_HASH_OFFSET);
        let hash_a: u32 = hash_string(crypt_table, file_name, HASH_TABLE_HASH_A_OFFSET);
        let hash_b: u32 = hash_string(crypt_table, file_name, HASH_TABLE_HASH_B_OFFSET);

        let hash_start = hash % self.header.hash_table_size;
        let mut hash_pos = hash_start;

        let mut candidate: MPQHashTableEntry = get_entry_at(&self.hash_table, hash_pos);

        while candidate.exists() {
            if candidate._name1 == hash_a && candidate._name2 == hash_b {
                return Some(candidate);
            } else {
                hash_pos = (hash_pos + 1) % self.header.hash_table_size;
                candidate = get_entry_at(&self.hash_table, hash_pos);
            }

            if hash_pos == hash_start {
                break;
            }
        }

        None
    }

    pub fn get_block_table_entry_at(&self, block_number: u32) -> MPQBlockTableEntry {
        let mut reader = Cursor::new(&self.block_table);
        reader
            .seek(SeekFrom::Start(
                block_number as u64 * BLOCK_TABLE_ENTRY_SIZE as u64,
            ))
            .unwrap();
        let entry: MPQBlockTableEntry = reader.read_le().unwrap();
        entry
    }

    pub fn get_file_data(&mut self, entry: &MPQBlockTableEntry) -> Result<Vec<u8>, std::io::Error> {
        let mut buffer = Vec::new();
        buffer.resize(entry.compressed_file_size as usize, 0);
        self.underlying_file
            .seek(SeekFrom::Start(entry.file_pos as u64))?;
        self.underlying_file.read(&mut buffer)?;

        if buffer.len() < 4 {
            return Ok(Vec::new());
        }

        let max_multiple_of_4_in_buffer = buffer.len() - (buffer.len() % 4);
        let sector_offsets: Vec<u32> = cast_slice(&buffer[..max_multiple_of_4_in_buffer]).to_vec();

        let mut final_buffer: Vec<u8> = Vec::new();

        // Read sectors offsets one by one until we have gathered the expected amount of bytes
        let mut sector_index = 0;
        while sector_offsets[sector_index] < entry.compressed_file_size {
            let mut sector_buffer: Vec<u8> = Vec::new();
            sector_buffer.resize(
                sector_offsets[sector_index + 1] as usize - sector_offsets[sector_index] as usize,
                0,
            );
            self.underlying_file
                .seek(SeekFrom::Start(
                    entry.file_pos as u64 + sector_offsets[sector_index] as u64,
                ))
                .unwrap();
            self.underlying_file.read(&mut sector_buffer)?;

            let compression_flags = sector_buffer[0]; // FIXME: Verify why it's not
                                                      // sector_buffer[sector_index]
            trace!("compression_flags: {:#X}", compression_flags);

            sector_buffer.drain(0..1);
            let mut decompressed_sector = decompress(sector_buffer, compression_flags);

            final_buffer.append(&mut decompressed_sector);

            sector_index += 1;
        }

        Ok(final_buffer)
    }
}

#[binread]
#[derive(Debug)] // http://www.zezula.net/en/mpq/mpqformat.html
pub struct MPQHeader {
    pub signature: [u8; 4], // Must be "MPQ\x1A"
    pub _header_size: u32,
    pub _archive_size: u32,
    pub _format_version: u16,
    pub _block_size: u16,
    pub hash_table_offset: u32,  // From the beginning of the archive
    pub block_table_offset: u32, // From the beginning of the archive
    pub hash_table_size: u32,
    pub block_table_size: u32,
    pub _high_block_table_pos: u64,
    pub _hash_table_pos_high: u16,
    pub _block_table_pos_high: u16,
}

#[binread]
#[derive(Debug)]
pub struct MPQHashTableEntry {
    pub _name1: u32,
    pub _name2: u32,
    pub _locale: u16,
    pub _platform: u16,
    pub block_index: u32,
}

impl MPQHashTableEntry {
    pub fn exists(&self) -> bool {
        self.block_index != crate::constants::BLOCK_ENTRY_IS_FREE
    }
}

#[binread]
#[derive(Debug)]
pub struct MPQBlockTableEntry {
    pub file_pos: u32,
    pub compressed_file_size: u32,
    pub uncompressed_file_size: u32,
    pub flags: u32,
}
