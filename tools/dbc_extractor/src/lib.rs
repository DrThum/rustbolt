use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
};

use bytemuck::cast_slice;
use constants::{BLOCK_TABLE_ENTRY_SIZE, HASH_TABLE_ENTRY_SIZE};
use log::trace;
use models::MPQFile;

pub mod constants;
pub mod models;
pub mod utils;

// TODO:
// - Implement a CLI in main.rs to specify
//      - WoW base folder
//      - output directory
// - Implement MPQ chaining (sort by priority and stop whenever a file is found)
pub fn extract(mpq_path: &str, file_name: &str) -> Result<(), std::io::Error> {
    trace!("Preparing crypto table...");
    let mut crypt_table = [0_u32; 0x500];
    utils::crypto::prepare_crypt_table(&mut crypt_table);

    let mut mpq: MPQFile = open_archive(mpq_path, &crypt_table)?;

    let hash_table_entry = mpq.find_hash_table_entry(file_name, &crypt_table);

    let block_table_entry = mpq.get_block_table_entry_at(hash_table_entry.unwrap().block_index);

    let file_data = mpq.get_file_data(&block_table_entry)?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open("ChrRaces.dbc")
        .unwrap();

    file.write_all(&file_data).unwrap();
    Ok(())
}

fn open_archive(path: &str, crypt_table: &[u32; 0x500]) -> Result<MPQFile, std::io::Error> {
    let mut file = File::open(path)?;
    let mpq_header = utils::mpq::get_header(&mut file)?;

    let mut buffer: Vec<u8> = Vec::new();
    buffer.resize(
        mpq_header.hash_table_size as usize * HASH_TABLE_ENTRY_SIZE,
        0,
    );
    file.seek(SeekFrom::Start(mpq_header.hash_table_offset as u64))?;
    file.read(&mut buffer)?;

    let mut encrypted_hash_table: Vec<u32> = cast_slice(&buffer).to_vec();
    let key = utils::mpq::hash_string(&crypt_table, "(hash table)", 3);
    utils::crypto::decrypt_block_in_place(&mut encrypted_hash_table, key, &crypt_table);

    let decrypted_hash_table: Vec<u8> = cast_slice(&encrypted_hash_table).to_vec();

    let mut buffer: Vec<u8> = Vec::new();
    buffer.resize(
        mpq_header.block_table_size as usize * BLOCK_TABLE_ENTRY_SIZE,
        0,
    );
    file.seek(SeekFrom::Start(mpq_header.block_table_offset as u64))?;
    file.read(&mut buffer)?;

    let mut encrypted_block_table: Vec<u32> = cast_slice(&buffer).to_vec();
    let key = utils::mpq::hash_string(&crypt_table, "(block table)", 3);
    utils::crypto::decrypt_block_in_place(&mut encrypted_block_table, key, &crypt_table);

    let decrypted_block_table: Vec<u8> = cast_slice(&encrypted_block_table).to_vec();

    Ok(MPQFile::new(
        mpq_header,
        decrypted_hash_table,
        decrypted_block_table,
        file,
    ))
}
