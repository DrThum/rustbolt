use binrw::io::Cursor;

use super::{FileChunk, FileType, MVER};

pub use self::chunks::*;

pub mod chunks;

pub struct WMO {}

pub struct RootWMO {
    pub group_count: u32,
}

pub struct WMOGroup {}

impl WMO {
    pub fn parse_root(raw: &Vec<u8>) -> Option<RootWMO> {
        let mut reader = Cursor::new(raw);
        let _mver: Box<MVER> = FileChunk::read_as(FileType::WMO, &mut reader);
        let mohd: Box<MOHD> = FileChunk::read_as(FileType::WMO, &mut reader);

        Some(RootWMO {
            group_count: mohd.group_count,
        })
    }

    pub fn parse_group(raw: Vec<u8>) -> Option<WMOGroup> {
        let mut reader = Cursor::new(&raw);
        let _mver: Box<MVER> = FileChunk::read_as(FileType::WMO, &mut reader);
        let position = reader.position();
        let mogp: Box<MOGP> = FileChunk::read_as(FileType::WMO, &mut reader);
        // Go back to where we were before + the actual MOGP size to read the rest of the chunks
        reader.set_position(
            position + std::mem::size_of::<MOGP>() as u64 + 8, /* magic + size */
        );
        let _mopy: Box<MOPY> = FileChunk::read_as(FileType::WMO, &mut reader);
        let _movi: Box<MOVI> = FileChunk::read_as(FileType::WMO, &mut reader);
        let _movt: Box<MOVT> = FileChunk::read_as(FileType::WMO, &mut reader);
        let _monr: Box<MONR> = FileChunk::read_as(FileType::WMO, &mut reader);
        let _motv: Box<MOTV> = FileChunk::read_as(FileType::WMO, &mut reader);
        let _moba: Box<MOBA> = FileChunk::read_as(FileType::WMO, &mut reader);
        if mogp.flags.contains(WMOGroupFlags::HasBSPNodes) {
            let _mobn: Box<MOBN> = FileChunk::read_as(FileType::WMO, &mut reader);
            let _mobr: Box<MOBR> = FileChunk::read_as(FileType::WMO, &mut reader);
        }
        if mogp.flags.contains(WMOGroupFlags::HasWater) {
            let _mliq: Box<MLIQ> = FileChunk::read_as(FileType::WMO, &mut reader);
        }
        // TODO: More optional chunks are possible here

        Some(WMOGroup {})
    }
}
