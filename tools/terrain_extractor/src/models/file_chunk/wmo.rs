use binrw::{binread, io::Cursor, BinReaderExt};
use shared::models::terrain_info::BoundingBox;

use super::{ColorData, FileChunk, FileType, TypedFileChunk, MVER};

pub struct WMO {}

impl WMO {
    pub fn parse_root(raw: &Vec<u8>) -> Option<Self> {
        let mut reader = Cursor::new(raw);
        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from WMO file");
        let _mver = chunk
            .as_typed(FileType::WMO)
            .downcast::<MVER>()
            .unwrap_or_else(|_| {
                panic!("expected MVER chunk, got {}", chunk.magic_str());
            });

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from WMO file");
        let _mohd = chunk
            .as_typed(FileType::WMO)
            .downcast::<MOHD>()
            .unwrap_or_else(|_| {
                panic!("expected MOHD chunk, got {}", chunk.magic_str());
            });
        // println!("{:?}", mohd);

        None
    }
}

#[binread]
#[allow(dead_code)]
#[derive(Debug)]
pub struct MOHD {
    textures_num: u32,
    groups_num: u32,
    portals_num: u32,
    lights_num: u32,
    doodad_names_num: u32,
    doodad_defs_num: u32,
    doodad_sets_num: u32,
    ambient_color: ColorData,
    wmo_areatable_wmo_id: i32, // Can be -1
    bounding_box: BoundingBox,
    flags: u32, // https://wowdev.wiki/WMO#MOHD_chunk
}

impl MOHD {
    pub fn parse(raw: &Vec<u8>) -> Result<MOHD, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let header: MOHD = reader.read_le()?;

        Ok(header)
    }
}

impl TypedFileChunk for MOHD {
    fn name(&self) -> &str {
        "MOHD"
    }

    fn content_as_string(&self) -> String {
        format!(
            "groups: {}, areatable id: {}, flags: {}",
            self.groups_num, self.wmo_areatable_wmo_id, self.flags
        )
    }
}
