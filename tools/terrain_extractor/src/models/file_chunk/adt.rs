use std::collections::HashMap;

use binrw::{binread, io::Cursor, BinReaderExt};
use bytemuck::cast_slice;
use enumflags2::{bitflags, BitFlags};
use log::error;
use shared::models::terrain_info::{
    LiquidFlags, LiquidTypeEntry, TerrainBlock, TerrainChunk, TerrainLiquidInfo, MAP_MAX_COORD,
};

use super::{BoundingBox, FileChunk, FileType, TypedFileChunk, Vector3, MVER};

pub struct ADT {
    mcnk_chunks: Vec<MCNK>,
    mwid_chunk: MWID,
    mwmo_chunk: MWMO,
    modf_chunk: MODF, // WMO placements
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
        let _mmdx = chunk
            .as_typed(FileType::ADT)
            .downcast::<MMDX>()
            .unwrap_or_else(|_| panic!("expected MMDX chunk, got {}", chunk.magic_str()));

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MMID>() {
            error!("expected MMID chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        let mwmo = chunk
            .as_typed(FileType::ADT)
            .downcast::<MWMO>()
            .unwrap_or_else(|_| panic!("expected MWMO chunk, got {}", chunk.magic_str()));

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        let mwid = chunk
            .as_typed(FileType::ADT)
            .downcast::<MWID>()
            .unwrap_or_else(|_| panic!("expected MWID chunk, got {}", chunk.magic_str()));

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        if let Err(_) = chunk.as_typed(FileType::ADT).downcast::<MDDF>() {
            error!("expected MDDF chunk, got {}", chunk.magic_str());
        }

        let chunk: FileChunk = reader
            .read_le()
            .expect("failed to read chunk from ADT file");
        let modf = chunk
            .as_typed(FileType::ADT)
            .downcast::<MODF>()
            .unwrap_or_else(|_| panic!("expected MODF chunk, got {}", chunk.magic_str()));

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

        Some(ADT {
            mcnk_chunks,
            mwid_chunk: *mwid,
            mwmo_chunk: *mwmo,
            modf_chunk: *modf,
        })
    }

    pub(crate) fn terrain_block(&self) -> TerrainBlock {
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

    pub(crate) fn list_wmos_to_extract(&self) -> Vec<&String> {
        self.modf_chunk
            .wmo_placements
            .iter()
            .map(|wmo_placement| {
                let mwmo_offset =
                    self.mwid_chunk.offsets[wmo_placement.mwid_index as usize] as usize;
                self.mwmo_chunk.filenames.get(&mwmo_offset).unwrap()
            })
            .collect()
    }
    // TODO WMO:
    // For each record in self.modf_chunk.wmo_placements
    // - Open the root wmo file (follow the path through MODF -> MWID -> MWMO)
    // - Read MOHD to get data (among other things, the number of groups)
    //     https://wowdev.wiki/WMO#MOHD_chunk
    // - For each wmo group
    //   - Read relevant data (to be defined)
    // - Build a mesh for the whole map? just for a chunk? how to handle cross-chunk situations?
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
#[derive(Debug)]
pub struct MMDX {
    filenames: Vec<String>,
}

impl MMDX {
    pub fn parse(raw: &Vec<u8>) -> Result<MMDX, binrw::Error> {
        let filenames: Vec<String> = raw
            .split(|&c| c == 0)
            .map(|ints| String::from_utf8(ints.to_vec()).unwrap())
            .filter(|str| !str.is_empty())
            .collect();

        Ok(MMDX { filenames })
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
    offsets: Vec<u8>,
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
#[derive(Debug)]
pub struct MWMO {
    // The key is the offset of the string from the start of the chunk
    pub filenames: HashMap<usize, String>,
}

impl MWMO {
    pub fn parse(raw: &Vec<u8>) -> Result<MWMO, binrw::Error> {
        let mut filenames: HashMap<usize, String> = HashMap::new();
        let mut buffer: Vec<u8> = Vec::new();
        let mut current_start_index = 0;

        raw.iter().enumerate().for_each(|(index, c)| {
            if *c == 0 {
                let as_string = String::from_utf8(buffer.to_vec()).unwrap();
                if !as_string.is_empty() {
                    filenames.insert(current_start_index, as_string);
                }
                buffer.clear();
                current_start_index = index + 1;
            } else {
                buffer.push(*c);
            }
        });

        Ok(MWMO { filenames })
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
#[derive(Debug)]
pub struct MWID {
    pub offsets: Vec<u32>,
}

impl MWID {
    pub fn parse(raw: &Vec<u8>) -> Result<MWID, binrw::Error> {
        Ok(MWID {
            offsets: cast_slice(&raw).to_vec(),
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
#[derive(Debug)]
pub struct MODF {
    wmo_placements: Vec<WmoPlacement>,
}

#[allow(dead_code)]
#[binread]
#[derive(Debug)]
pub struct WmoPlacement {
    mwid_index: u32,
    _uuid: u32,
    position: Vector3,
    rotation: Vector3,
    bounding_box: BoundingBox,
    flags: u16,
    doodad_mods_index: u16, // MODS chunk in WMO file
    _name_set: u16,
    _padding: u16,
}

impl MODF {
    pub fn parse(raw: &Vec<u8>) -> Result<MODF, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let mut wmo_placements: Vec<WmoPlacement> = Vec::new();
        while let Ok(mut placement) = reader.read_le::<WmoPlacement>() {
            // println!("{:?}", placement.bounding_box.min);
            // 1. Translate the position onto the World referential
            let converted_position = Vector3 {
                x: MAP_MAX_COORD - placement.bounding_box.min.x,
                y: placement.bounding_box.min.y,
                z: MAP_MAX_COORD - placement.bounding_box.min.z,
            };
            // println!("\t-> {converted_position:?}");

            // 3. Apply rotations
            // let converted_position = glm::rotate_x_vec4(
            //     &glm::make_vec4(&[
            //         converted_position.x,
            //         converted_position.y,
            //         converted_position.z,
            //         1.0,
            //     ]),
            //     f32::to_radians(placement.rotation.x),
            // );
            // let converted_position = glm::rotate_y_vec4(
            //     &glm::make_vec4(&[
            //         converted_position.x,
            //         converted_position.y,
            //         converted_position.z,
            //         1.0,
            //     ]),
            //     f32::to_radians(placement.rotation.y),
            // );
            // let converted_position = glm::rotate_z_vec4(
            //     &glm::make_vec4(&[
            //         converted_position.x,
            //         converted_position.y,
            //         converted_position.z,
            //         1.0,
            //     ]),
            //     f32::to_radians(placement.rotation.z),
            // );
            // println!("\t-> {converted_position:?}");

            // 2. Swap axes:
            // - World X is WMO Z
            // - World Y is WMO X
            // - World Z is WMO Y
            let converted_position = Vector3 {
                x: converted_position.z,
                y: converted_position.x,
                z: converted_position.y,
            };
            // println!("\t\t-> {converted_position:?}");

            placement.position = converted_position;
            wmo_placements.push(placement);
        }

        Ok(MODF { wmo_placements })
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

#[allow(dead_code)]
pub struct MCNK {
    header: MCNKHeader,
    height_map: [f32; 145], // Height relative to position_y in MCNKHeader
    liquid_info: Option<MCLQ>,
    m2_refs: Vec<u32>,
    wmo_refs: Vec<u32>,
}

impl MCNK {
    pub fn parse(raw: &Vec<u8>) -> Result<MCNK, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let header: MCNKHeader = reader.read_le()?;

        // Seek to the MCVT sub-chunk
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

        // Seek to the MCRF sub-chunk
        // Same logic applies for +8/-8 than the MCVT sub-chunk a few lines above
        reader.set_position((header.offset_mcrf - 8 + 8) as u64);
        let mut doodad_refs: Vec<u32> = Vec::new();
        for _ in 0..header.nb_doodad_refs {
            let doodad_index: u32 = reader.read_le()?;
            doodad_refs.push(doodad_index);
        }
        let mut map_object_refs: Vec<u32> = Vec::new();
        for _ in 0..header.nb_map_object_refs {
            let object_index: u32 = reader.read_le()?;
            map_object_refs.push(object_index);
        }

        Ok(MCNK {
            header,
            height_map,
            liquid_info,
            m2_refs: doodad_refs,
            wmo_refs: map_object_refs,
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
