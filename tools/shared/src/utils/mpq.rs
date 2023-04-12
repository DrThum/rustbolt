use binrw::{io::Cursor, BinReaderExt};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use crate::models::*;

pub fn get_header(file: &mut File) -> Result<MPQHeader, std::io::Error> {
    let mut buffer = [0; 1024];
    file.seek(SeekFrom::Start(0))?;
    file.read(&mut buffer)?;

    let mut reader = Cursor::new(buffer);
    let mpq_header: MPQHeader = reader.read_le().unwrap();
    if mpq_header.signature == [b'M', b'P', b'Q', b'\x1A'] {
        Ok(mpq_header)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "input file is not an MPQ archive",
        ))
    }
}

// http://www.zezula.net/en/mpq/techinfo.html#Hashes
pub fn hash_string(crypt_table: &[u32; 0x500], file_name: &str, hash_type: u32) -> u32 {
    let mut seed1: u32 = 0x7FED7FED;
    let mut seed2: u32 = 0xEEEEEEEE;

    for c in file_name.chars() {
        let ch = c.to_ascii_uppercase();

        let cr_idx: usize = usize::try_from((hash_type << 8) + (ch as u32)).unwrap();
        let crypt_base: u32 = crypt_table[cr_idx];

        seed1 = crypt_base ^ (seed1.wrapping_add(seed2));
        seed2 = (ch as u32)
            .wrapping_add(seed1.wrapping_add(seed2.wrapping_add(seed2 << 5).wrapping_add(3)));
    }

    seed1
}
