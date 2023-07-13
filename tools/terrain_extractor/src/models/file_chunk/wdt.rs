use binrw::io::Cursor;

use crate::models::file_chunk::FileChunk;

pub use self::chunks::*;

use super::{FileType, MVER};

pub mod chunks;

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
        let _mver: Box<MVER> = FileChunk::read_as(FileType::WDT, &mut reader);
        let _mphd: Box<MPHD> = FileChunk::read_as(FileType::WDT, &mut reader);
        let main_chunk: Box<MAIN> = FileChunk::read_as(FileType::WDT, &mut reader);
        // TODO: For non-terrain-based maps, there is one MWMO and one MODF chunk here
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
    }
}
