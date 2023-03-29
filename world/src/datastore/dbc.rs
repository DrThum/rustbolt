use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    mem::size_of,
};

use bytemuck::cast_slice;

use super::{data_types::DbcTypedRecord, DbcStore};

pub struct Dbc {
    _header: DbcHeader,
    records: Vec<DbcRecord>,
    _strings: DbcStringBlock,
}

impl Dbc {
    pub fn parse(file_path: String) -> Result<Dbc, std::io::Error> {
        let mut file = File::open(file_path)?;

        let header = DbcHeader::read(&mut file)?;
        let records = DbcRecord::read(&mut file, &header)?;
        let strings = DbcStringBlock::read(&mut file, &header)?;

        Ok(Dbc {
            _header: header,
            records,
            _strings: strings,
        })
    }

    pub fn as_store<T: DbcTypedRecord>(&self) -> DbcStore<T> {
        self.records
            .iter()
            .map(|dbc_record| T::from_record(dbc_record))
            .collect()
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

pub struct DbcStringBlock {
    _strings: Vec<String>,
}

impl DbcStringBlock {
    fn read(file: &mut File, header: &DbcHeader) -> Result<DbcStringBlock, std::io::Error> {
        let mut buffer = Vec::new();
        buffer.resize(header.string_block_size as usize, 0);

        file.seek(SeekFrom::End(-(header.string_block_size as i64)))?;
        file.read(&mut buffer)?;

        let strings: Vec<String> = buffer
            .into_iter()
            .fold(Vec::new(), |mut acc, x| {
                if x == 0 || acc.is_empty() {
                    acc.push(Vec::new());
                } else {
                    acc.last_mut().unwrap().push(x);
                }
                acc
            })
            .into_iter()
            .map(|bytes| std::str::from_utf8(&bytes).unwrap().to_owned())
            .collect();

        Ok(DbcStringBlock { _strings: strings })
    }
}

pub struct LocalizedString {
    locales: [String; 16],
    bitmask: u32, // Not sure that it's 4 bytes
}
