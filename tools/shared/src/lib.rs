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
pub mod utils {
    pub mod compression;
    pub mod crypto;
    pub mod mpq;
}

pub fn extract_files(
    client_data_dir: &str,
    files_to_extract: Vec<&str>,
    output_dir: &str,
) -> Result<(), std::io::Error> {
    let (mut mpqs, crypt_table) = open_mpqs(client_data_dir)?;

    for file_to_extract in files_to_extract {
        for mpq in &mut mpqs {
            let maybe_file_data = mpq
                .find_hash_table_entry(file_to_extract, &crypt_table)
                .map(|hash_table_entry| {
                    let block_table_entry =
                        mpq.get_block_table_entry_at(hash_table_entry.block_index);
                    mpq.get_file_data(&block_table_entry).unwrap()
                });

            if let Some(file_data) = maybe_file_data {
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(format!(
                        "{}/{}",
                        output_dir,
                        file_to_extract.rsplit_once('\\').unwrap().1
                    ))
                    .unwrap();

                file.write_all(&file_data).unwrap();
                break;
            }
        }
    }

    Ok(())
}

fn open_mpqs(client_data_dir: &str) -> Result<(Vec<MPQFile>, [u32; 0x500]), std::io::Error> {
    let mpqs_by_priority: Vec<String> = vec![
        "/frFR/patch-frFR-2.MPQ",
        "/frFR/patch-frFR.MPQ",
        "/frFR/base-frFR.MPQ",
        "/frFR/speech-frFR.MPQ",
        "/frFR/locale-frFR.MPQ",
        "/patch-2.MPQ",
        "/patch.MPQ",
        "/common.MPQ",
    ]
    .into_iter()
    .map(|suffix| {
        let mut full_path = client_data_dir.to_owned();
        full_path.push_str(suffix);
        full_path
    })
    .collect();

    trace!("Preparing crypto table...");
    let mut crypt_table = [0_u32; 0x500];
    utils::crypto::prepare_crypt_table(&mut crypt_table);

    Ok((
        mpqs_by_priority
            .into_iter()
            .map(|mpq_path| {
                open_archive(&mpq_path, &crypt_table)
                    .expect(&format!("{} not found, check your WoW install.", mpq_path))
            })
            .collect(),
        crypt_table,
    ))
}

fn open_archive(path: &str, crypt_table: &[u32; 0x500]) -> Result<MPQFile, std::io::Error> {
    let mut file = File::open(path).expect(&format!(
        "Required MPQ archive {} not found. Check your WoW install.",
        path
    ));
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
