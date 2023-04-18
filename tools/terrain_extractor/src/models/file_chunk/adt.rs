use binrw::{binread, io::Cursor, BinReaderExt};
use enumflags2::{bitflags, BitFlags};
use log::error;
use shared::models::terrain_info::{
    LiquidFlags, LiquidTypeEntry, TerrainBlock, TerrainChunk, TerrainLiquidInfo,
};

use super::{FileChunk, FileType, TypedFileChunk, MVER};

pub struct ADT {
    mcnk_chunks: Vec<MCNK>,
}

// An ADT has 16x16 = 256 map chunks
impl ADT {
    pub fn parse(raw: &Vec<u8>) -> Option<ADT> {
        let mut reader = Cursor::new(raw);
        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MVER>() {
            error!("expected MVER chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MHDR>() {
            error!("expected MHDR chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MCIN>() {
            error!("expected MCIN chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MTEX>() {
            error!("expected MTEX chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MMDX>() {
            error!("expected MMDX chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MMID>() {
            error!("expected MMID chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MWMO>() {
            error!("expected MWMO chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MWID>() {
            error!("expected MWID chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MDDF>() {
            error!("expected MDDF chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MODF>() {
            error!("expected MODF chunk, got {}", chunk.magic_str());
        }

        let mut mcnk_chunks: Vec<MCNK> = Vec::new();
        for _i in 0..256 {
            let chunk: FileChunk = reader
                .read_le()
                .expect("failed to read chunk from ADT file");
            if let Ok(mcnk) = chunk.as_typed(FileType::ADT).downcast::<MCNK>() {
                mcnk_chunks.push(*mcnk);
            } else {
                error!("expected MCNK chunk, got {}", chunk.magic_str());
            }
        }

        assert!(
            mcnk_chunks.len() == 256,
            "expected 256 MCNK chunks, found {}",
            mcnk_chunks.len()
        );

        Some(ADT { mcnk_chunks })
    }

    pub(crate) fn to_terrain_block(&self) -> TerrainBlock {
        let terrain_chunks: Vec<TerrainChunk> = self
            .mcnk_chunks
            .iter()
            .map(|mcnk| {
                let liquid_info: Option<TerrainLiquidInfo> = mcnk
                    .liquid_info
                    .as_ref()
                    .map(|lq| lq.to_terrain_info(&mcnk));

                TerrainChunk::new(
                    mcnk.header.index_y,
                    mcnk.header.index_x,
                    mcnk.header.area_id,
                    mcnk.header.position_z,
                    mcnk.header.holes,
                    mcnk.height_map,
                    liquid_info,
                )
            })
            .collect();

        TerrainBlock::new(terrain_chunks)
    }
}

#[binread]
#[allow(dead_code)]
pub struct MHDR {
    flags: u32,
    offset_mcin: u32,
    offset_mtex: u32,
    offset_mmdx: u32,
    offset_mmid: u32,
    offset_mwmo: u32,
    offset_mwid: u32,
    offset_mddf: u32,
    offset_modf: u32,
    offset_mfbo: u32,
    offset_mh2o: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    data4: u32,
    data5: u32,
}

impl MHDR {
    pub fn parse(raw: &Vec<u8>) -> Result<MHDR, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let mhdr: MHDR = reader.read_le()?;
        Ok(mhdr)
    }
}

impl TypedFileChunk for MHDR {
    fn name(&self) -> &str {
        "MHDR"
    }

    fn content_as_string(&self) -> String {
        format!("flags: {:X}", self.flags)
    }
}

#[binread]
#[allow(dead_code)]
pub struct SMChunkInfo {
    offset: u32,
    size: u32,
    flags: u32,
    pad: [u8; 4],
}

#[binread]
#[allow(dead_code)]
pub struct MCIN {
    chunks: [SMChunkInfo; 16 * 16],
}

impl MCIN {
    pub fn parse(raw: &Vec<u8>) -> Result<MCIN, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let mcin: MCIN = reader.read_le()?;
        Ok(mcin)
    }
}

impl TypedFileChunk for MCIN {
    fn name(&self) -> &str {
        "MCIN"
    }

    fn content_as_string(&self) -> String {
        "".to_owned()
    }
}

#[allow(dead_code)]
pub struct MTEX {
    filenames: Vec<u8>, // No need to parse this one
}

impl MTEX {
    pub fn parse(raw: &Vec<u8>) -> Result<MTEX, binrw::Error> {
        Ok(MTEX {
            filenames: raw.clone(),
        })
    }
}

impl TypedFileChunk for MTEX {
    fn name(&self) -> &str {
        "MTEX"
    }

    fn content_as_string(&self) -> String {
        "".to_owned()
    }
}

#[allow(dead_code)]
pub struct MMDX {
    filenames: Vec<u8>, // No need to parse this one
}

impl MMDX {
    pub fn parse(raw: &Vec<u8>) -> Result<MMDX, binrw::Error> {
        Ok(MMDX {
            filenames: raw.clone(),
        })
    }
}

impl TypedFileChunk for MMDX {
    fn name(&self) -> &str {
        "MMDX"
    }

    fn content_as_string(&self) -> String {
        "".to_owned()
    }
}

#[allow(dead_code)]
pub struct MMID {
    offsets: Vec<u8>, // No need to parse this one
}

impl MMID {
    pub fn parse(raw: &Vec<u8>) -> Result<MMID, binrw::Error> {
        Ok(MMID {
            offsets: raw.clone(),
        })
    }
}

impl TypedFileChunk for MMID {
    fn name(&self) -> &str {
        "MMID"
    }

    fn content_as_string(&self) -> String {
        "".to_owned()
    }
}

#[allow(dead_code)]
pub struct MWMO {
    filenames: Vec<u8>, // No need to parse this one
}

impl MWMO {
    pub fn parse(raw: &Vec<u8>) -> Result<MWMO, binrw::Error> {
        Ok(MWMO {
            filenames: raw.clone(),
        })
    }
}

impl TypedFileChunk for MWMO {
    fn name(&self) -> &str {
        "MWMO"
    }

    fn content_as_string(&self) -> String {
        "".to_owned()
    }
}

#[allow(dead_code)]
pub struct MWID {
    offsets: Vec<u8>, // No need to parse this one
}

impl MWID {
    pub fn parse(raw: &Vec<u8>) -> Result<MWID, binrw::Error> {
        Ok(MWID {
            offsets: raw.clone(),
        })
    }
}

impl TypedFileChunk for MWID {
    fn name(&self) -> &str {
        "MWID"
    }

    fn content_as_string(&self) -> String {
        "".to_owned()
    }
}

#[allow(dead_code)]
pub struct MDDF {
    data: Vec<u8>,
}

impl MDDF {
    pub fn parse(raw: &Vec<u8>) -> Result<MDDF, binrw::Error> {
        Ok(MDDF { data: raw.clone() })
    }
}

impl TypedFileChunk for MDDF {
    fn name(&self) -> &str {
        "MDDF"
    }

    fn content_as_string(&self) -> String {
        "".to_owned()
    }
}

#[allow(dead_code)]
pub struct MODF {
    data: Vec<u8>,
}

impl MODF {
    pub fn parse(raw: &Vec<u8>) -> Result<MODF, binrw::Error> {
        Ok(MODF { data: raw.clone() })
    }
}

impl TypedFileChunk for MODF {
    fn name(&self) -> &str {
        "MODF"
    }

    fn content_as_string(&self) -> String {
        "".to_owned()
    }
}

#[allow(dead_code)]
#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum MCNKFlags {
    HasMCSH = 0x0001,
    Impass = 0x0002,
    LiquidRiver = 0x0004,
    LiquidOcean = 0x0008,
    LiquidMagmaOrSlime = 0x0010,
    HasMCCV = 0x0020,
}

#[binread]
#[allow(dead_code)]
pub struct MCNKHeader {
    #[br(map = |raw: u32| unsafe { BitFlags::from_bits_unchecked(raw) })]
    flags: BitFlags<MCNKFlags>,
    index_x: u32,
    index_y: u32,
    nb_layers: u32,
    nb_doodad_refs: u32,
    offset_mcvt: u32,
    offset_mcnr: u32,
    offset_mcly: u32,
    offset_mcrf: u32,
    offset_mcal: u32,
    size_mcal: u32,
    offset_mcsh: u32,
    size_mcsh: u32,
    area_id: u32,
    nb_map_object_refs: u32,
    holes: u32,
    s: [u16; 2],
    data1: u32,
    data2: u32,
    data3: u32,
    pred_tex: u32,
    nb_effect_doodad: u32,
    offset_mcse: u32,
    nb_sound_emitters: u32,
    offset_mclq: u32,
    size_mclq: u32,
    position_x: f32,
    position_y: f32,
    position_z: f32,
    offset_mccv: u32,
}

#[binread]
#[allow(dead_code)]
pub struct MCNK {
    header: MCNKHeader,
    height_map: [f32; 145], // Height relative to position_y in MCNKHeader
    liquid_info: Option<MCLQ>,
}

impl MCNK {
    pub fn parse(raw: &Vec<u8>) -> Result<MCNK, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let header: MCNKHeader = reader.read_le()?;

        // Seek to the MCVT chunk
        // Technically we have to do - 8 because the offset is from the
        // beginning of the chunk and our raw vector starts after 'magic'
        // and 'size' (see FileChunk), then + 8 to skip the sub-chunk header
        // of size 8 (magic: u32 + size: u32), both of which cancel themselves
        reader.set_position((header.offset_mcvt - 8 + 8) as u64);
        let height_map: [f32; 145] = reader.read_le()?;

        let liquid_info: Option<MCLQ> = if header.size_mclq == 8 {
            None
        } else {
            reader.set_position((header.offset_mclq - 8 + 8) as u64); // Same logic as before
            let mclq: MCLQ = reader.read_le()?;
            Some(mclq)
        };

        Ok(MCNK {
            header,
            height_map,
            liquid_info,
        })
    }
}

impl TypedFileChunk for MCNK {
    fn name(&self) -> &str {
        "MCNK"
    }

    fn content_as_string(&self) -> String {
        "".to_owned()
    }
}

#[allow(dead_code)]
pub enum MCLQFlags {
    Ocean = 1,
    Slime = 3,
    River = 4,
    Magma = 6,
    DontRender = 0xF, // Set height to something like -1000
    Unk = 0x40,
    Fatigue = 0x80,
}

#[binread]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct LiquidVertex {
    unk1: u16, // Maybe color or transparency?
    unk2: u16, // Maybe color or transparency?
    pub height: f32,
}

#[binread]
#[allow(dead_code)]
pub struct MCLQ {
    range_min: f32, // Liquid is above this height
    range_max: f32, // Liquid is below this height
    pub height_map: [LiquidVertex; 9 * 9],
    pub tiles_flags: [u8; 8 * 8], // Each element refers to a tile within the previous 9 * 9 grid
                                  // (see MCLQFlags)
}

impl MCLQ {
    pub fn to_terrain_info(&self, mcnk: &MCNK) -> TerrainLiquidInfo {
        let mut terrain_liquid_info_flags: BitFlags<LiquidFlags> = BitFlags::empty();

        let liquid_entry: LiquidTypeEntry = match mcnk.header.flags {
            flags if flags.contains(MCNKFlags::LiquidRiver) => {
                terrain_liquid_info_flags.insert(LiquidFlags::Water);
                LiquidTypeEntry::Water
            }
            flags if flags.contains(MCNKFlags::LiquidOcean) => {
                terrain_liquid_info_flags.insert(LiquidFlags::Ocean);
                LiquidTypeEntry::Ocean
            }
            flags if flags.contains(MCNKFlags::LiquidMagmaOrSlime) => {
                terrain_liquid_info_flags.insert(LiquidFlags::Magma);
                LiquidTypeEntry::Magma
            }
            _ => LiquidTypeEntry::NoWater,
        };

        // If at least one tile has LiquidFlags::Fatigue, add DarkWater to the chunk
        if self
            .tiles_flags
            .iter()
            .any(|&tile| (tile & MCLQFlags::Fatigue as u8 != 0))
        {
            terrain_liquid_info_flags.insert(LiquidFlags::DarkWater);
        }

        TerrainLiquidInfo::new(
            liquid_entry,
            terrain_liquid_info_flags,
            self.height_map.clone().map(|elm| elm.height),
            self.tiles_flags.clone().map(|tile| tile != 0xF),
        )
    }
}
