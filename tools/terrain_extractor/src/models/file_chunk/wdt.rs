use binrw::{binread, io::Cursor};
use binrw::{BinReaderExt, NullString};
use log::error;

use crate::models::file_chunk::FileChunk;

use super::{FileType, TypedFileChunk, MVER};

#[derive(Debug)]
pub struct Coordinates {
    pub row: usize,
    pub col: usize,
}

pub struct WDT {
    pub map_chunks: Vec<Coordinates>,
}

impl WDT {
    pub fn parse(raw: &Vec<u8>) -> Option<WDT> {
        let mut reader = Cursor::new(raw);
        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from WDT file");
        if let Err(_) = chunk.as_typed(FileType::WDT).downcast::<MVER>() {
            error!("expected MVER chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from WDT file");
        if let Err(_) = chunk.as_typed(FileType::WDT).downcast::<MPHD>() {
            error!("expected MPHD chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from WDT file");
        if let Ok(main_chunk) = chunk.as_typed(FileType::WDT).downcast::<MAIN>() {
            // let chunk: FileChunk = reader.read_le().expect("failed to read chunk from WDT file");
            // if let Err(_) = chunk.as_typed().downcast::<MWMO>() {
            //     error!("expected MWMO chunk, got {}", chunk.magic_str());
            // }

            let mut map_chunks: Vec<Coordinates> = Vec::new();
            for (index, area_info) in main_chunk.areas.iter().enumerate() {
                if area_info.exists {
                    map_chunks.push(Coordinates {
                        row: index / 64,
                        col: index % 64,
                    });
                }
            }

            return Some(WDT { map_chunks });
        } else {
            error!("expected MAIN chunk, got {}", chunk.magic_str());
            return None;
        }
    }
}

pub struct MPHD {
    flags: u32,
    _unk: u32,
    _unused: Vec<u32>,
}

impl MPHD {
    pub fn parse(raw: &Vec<u8>) -> Result<MPHD, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let data: [u32; 8] = reader.read_le()?;

        Ok(MPHD {
            flags: data[0],
            _unk: data[1],
            _unused: data[..2].to_vec(),
        })
    }
}

impl TypedFileChunk for MPHD {
    fn name(&self) -> &str {
        "MPHD"
    }

    fn content_as_string(&self) -> String {
        format!("flags {:X}", self.flags)
    }
}

#[binread]
#[derive(Debug)]
pub struct SMAreaInfo {
    #[br(map = |x: u32| if x == 1 { true } else { false })]
    exists: bool,
    _data1: u32, // Always 0
}

pub struct MAIN {
    areas: [SMAreaInfo; 64 * 64],
}

impl MAIN {
    pub fn parse(raw: &Vec<u8>) -> Result<MAIN, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let areas: [SMAreaInfo; 64 * 64] = reader.read_le()?;

        Ok(MAIN { areas })
    }
}

impl TypedFileChunk for MAIN {
    fn name(&self) -> &str {
        "MAIN"
    }

    fn content_as_string(&self) -> String {
        let num_existing = self.areas.iter().filter(|a| a.exists).count();
        format!("number of areas: {}", num_existing)
    }
}

pub struct MWMO {
    wmo_name: Option<NullString>,
}

impl MWMO {
    pub fn parse(raw: &Vec<u8>) -> Result<MWMO, binrw::Error> {
        let mut wmo_name: Option<NullString> = None;

        if !raw.is_empty() {
            let mut reader = Cursor::new(raw);
            let name: NullString = reader.read_le()?;
            wmo_name = Some(name);
        }

        Ok(MWMO { wmo_name })
    }
}

impl TypedFileChunk for MWMO {
    fn name(&self) -> &str {
        "MWMO"
    }

    fn content_as_string(&self) -> String {
        format!("wmo: {:?}", self.wmo_name)
    }
}
