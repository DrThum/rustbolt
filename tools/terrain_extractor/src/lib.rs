use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    mem::size_of,
};

use bytemuck::cast_slice;
use models::file_chunk::{adt::ADT, wdt::WDT};
use shared::models::terrain_info::TerrainBlock;

mod models {
    pub mod file_chunk;
}

// Here, we implement a simplified DBC parsing to retrieve just the internal names of the maps. No
// need of the full implementation from the world crate.
pub fn get_all_map_names(map_dbc_path: &str) -> Result<Vec<String>, std::io::Error> {
    const SIZE_OF_HEADER: usize = 5 * size_of::<u32>();

    let mut file = File::open(map_dbc_path)?;

    let mut buffer = Vec::new();
    buffer.resize(SIZE_OF_HEADER, 0); // Magic + Header

    file.read(&mut buffer)?;

    assert!(
        buffer[..4] == [b'W', b'D', b'B', b'C'],
        "Provided Map.dbc is not a valid DBC file"
    );

    let buffer: Vec<u32> = cast_slice(&buffer).to_vec();
    let record_count = buffer[1];
    let record_size = buffer[3];
    let string_block_size = buffer[4];

    // Gather string offsets from the record (internal name is the second field of each record -> offset 4)
    let mut string_offsets: Vec<u32> = Vec::new();
    file.seek(SeekFrom::Start((SIZE_OF_HEADER + 4) as u64))?;
    for _i in 0..record_count {
        let mut buffer: [u8; 4] = [0_u8; 4];
        file.read(&mut buffer)?;

        string_offsets.push(u32::from_le_bytes(buffer));

        file.seek(SeekFrom::Current((record_size - 4) as i64))?; // Advance to the next record
    }

    // Read the whole string block
    let mut buffer = Vec::new();
    buffer.resize(string_block_size as usize, 0);
    file.seek(SeekFrom::End(-(string_block_size as i64)))?;
    file.read(&mut buffer)?;

    let mut strings: Vec<String> = Vec::new();
    strings.reserve_exact(record_count as usize);

    for offset in string_offsets.iter() {
        let offset = *offset as usize;
        let slice = &buffer[offset..];
        let str_end_index = slice.iter().position(|&c| c == 0).unwrap();

        let slice = &buffer[offset..(offset + str_end_index)];
        strings.push(std::str::from_utf8(&slice).unwrap().to_owned());
    }

    Ok(strings)
}

pub fn read_wdt(raw: &Vec<u8>) -> Option<WDT> {
    if !raw.is_empty() {
        WDT::parse(raw)
    } else {
        None
    }
}

pub fn read_adt(raw: &Vec<u8>) -> Option<TerrainBlock> {
    if !raw.is_empty() {
        ADT::parse(raw).map(|adt| adt.to_terrain_block())
    } else {
        None
    }
}
