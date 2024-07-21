use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    mem::size_of,
};

use bytemuck::cast_slice;
use indicatif::ProgressBar;

use super::{
    data_types::{DbcTypedRecord, GameTableTypedRecord},
    DbcStore, GameTableStore,
};

pub struct Dbc {
    header: DbcHeader,
    records: Vec<DbcRecord>,
    strings: DbcStringBlock,
}

impl Dbc {
    pub fn parse(file_path: String) -> Result<Dbc, std::io::Error> {
        let mut file = File::open(file_path)?;

        let header = DbcHeader::read(&mut file)?;
        let records = DbcRecord::read(&mut file, &header)?;
        let strings = DbcStringBlock::read(&mut file, &header)?;

        Ok(Dbc {
            header,
            records,
            strings,
        })
    }

    pub fn as_store<T: DbcTypedRecord>(&self, bar: &ProgressBar) -> DbcStore<T> {
        self.records
            .iter()
            .map(|dbc_record| {
                let res = T::from_record(dbc_record, &self.strings);
                bar.inc(1);
                res
            })
            .collect()
    }

    pub fn as_gt_store<T: GameTableTypedRecord>(&self, bar: &ProgressBar) -> GameTableStore<T> {
        self.records
            .iter()
            .map(|dbc_record| {
                let res = T::from_record(dbc_record);
                bar.inc(1);
                res
            })
            .collect()
    }

    pub fn length(&self) -> u32 {
        self.header.record_count
    }
}

pub struct DbcHeader {
    record_count: u32,
    _field_count: u32, // Field count per record
    record_size: u32,
    string_block_size: u32,
}

impl DbcHeader {
    pub fn read(file: &mut File) -> Result<DbcHeader, std::io::Error> {
        let mut buffer = Vec::new();
        buffer.resize(size_of::<u32>() + size_of::<DbcHeader>(), 0); // Magic + Header

        file.seek(SeekFrom::Start(0))?;
        file.read(&mut buffer)?;

        if buffer[..4] == [b'W', b'D', b'B', b'C'] {
            let buffer: Vec<u32> = cast_slice(&buffer).to_vec();
            Ok(DbcHeader {
                record_count: buffer[1],
                _field_count: buffer[2],
                record_size: buffer[3],
                string_block_size: buffer[4],
            })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "input file is not a valid DBC file",
            ))
        }
    }
}

pub struct DbcRecord {
    pub fields: Vec<DbcValue>,
}

impl DbcRecord {
    fn read(file: &mut File, header: &DbcHeader) -> Result<Vec<DbcRecord>, std::io::Error> {
        let mut buffer = Vec::new();
        buffer.resize(header.record_size as usize, 0);

        file.seek(SeekFrom::Start(
            (size_of::<u32>() + size_of::<DbcHeader>()) as u64,
        ))?;

        let mut records: Vec<DbcRecord> = Vec::new();
        for _i in 0..header.record_count {
            file.read(&mut buffer)?;

            let fields: Vec<u32> = cast_slice(&buffer).to_vec();
            let fields: Vec<DbcValue> =
                fields.into_iter().map(|u| DbcValue { as_u32: u }).collect();
            let record = DbcRecord { fields };
            records.push(record);
        }

        Ok(records)
    }
}

pub union DbcValue {
    pub as_u32: u32,
    pub as_i32: i32,
    pub as_f32: f32,
}

#[derive(Debug)]
pub struct DbcStringBlock {
    pub raw_characters: Vec<u8>,
}

impl DbcStringBlock {
    fn read(file: &mut File, header: &DbcHeader) -> Result<DbcStringBlock, std::io::Error> {
        let mut buffer = Vec::new();
        buffer.resize(header.string_block_size as usize, 0);

        file.seek(SeekFrom::End(-(header.string_block_size as i64)))?;
        file.read(&mut buffer)?;

        Ok(DbcStringBlock {
            raw_characters: buffer,
        })
    }

    pub fn get(&self, offset: usize) -> Option<String> {
        if offset > self.raw_characters.len() {
            None
        } else {
            let slice = &self.raw_characters[offset..];
            let str_end_index = slice.iter().position(|&c| c == 0).unwrap();

            let slice = &self.raw_characters[offset..(offset + str_end_index)];
            Some(std::str::from_utf8(slice).unwrap().to_owned())
        }
    }
}
