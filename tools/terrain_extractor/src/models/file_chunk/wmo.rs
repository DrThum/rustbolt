use binrw::{binread, io::Cursor, BinReaderExt};
use enumflags2::{bitflags, BitFlags};
use shared::models::terrain_info::{BoundingBox, Vector3};

use super::{FileChunk, FileType, TypedFileChunk, MVER};

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

#[binread]
#[allow(dead_code)]
#[derive(Debug)]
pub struct MOHD {
    texture_count: u32,
    group_count: u32,
    portal_count: u32,
    light_count: u32,
    doodad_name_count: u32,
    doodad_def_count: u32,
    doodad_set_count: u32,
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
            self.group_count, self.wmo_areatable_wmo_id, self.flags
        )
    }
}

#[allow(dead_code)]
#[binread]
#[derive(Debug)]
pub struct ColorData {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

#[binread]
#[allow(dead_code)]
#[derive(Debug)]
// This chunk actually contains many other chunks, so the size is way bigger than the data we are
// storing in it. To avoid implementing sub-chunk management, we will just reset the read cursor
// after reading this one and read the "sub-chunks" as main chunks.
pub struct MOGP {
    _group_name_offset_in_root: i32,
    _descr_group_name_offset_in_root: i32,
    #[br(map = |raw: u32| unsafe { BitFlags::from_bits_unchecked(raw) })]
    pub flags: BitFlags<WMOGroupFlags>,
    _bounding_box: BoundingBox,
    _portal_start: u16, // Offset into MOPR
    _portal_count: u16, // Number of MOPR items used after portal_start
    _batches_a: u16,
    _batches_b: u16,
    _batches_c: u16,
    _batches_d: u16,
    _fog: [u8; 4],
    _liquid: u32,
    _wmo_group_id: u32,
    _padding: [u32; 2],
}

impl TypedFileChunk for MOGP {
    fn name(&self) -> &str {
        "MOGP"
    }
}

impl MOGP {
    pub fn parse(raw: &Vec<u8>) -> Result<MOGP, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let mogp: MOGP = reader.read_le()?;

        Ok(mogp)
    }
}

pub struct MOPY {
    pub _entries: Vec<MOPYEntry>,
}

#[binread]
pub struct MOPYEntry {
    _flags: u8,
    _material_id: u8, // Index into MOMT chunk
}

impl MOPY {
    pub fn parse(raw: &Vec<u8>) -> Result<MOPY, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let mut entries = Vec::new();
        while let Ok(entry) = reader.read_le() {
            entries.push(entry);
        }

        Ok(MOPY { _entries: entries })
    }
}

impl TypedFileChunk for MOPY {
    fn name(&self) -> &str {
        "MOPY"
    }
}

pub struct MOVI {
    _indices: Vec<TriangleIndices>,
}

pub struct TriangleIndices {
    _index0: u16,
    _index1: u16,
    _index2: u16,
}

impl MOVI {
    pub fn parse(raw: &Vec<u8>) -> Result<MOVI, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let mut indices = Vec::new();
        while let Ok(triangle_indices) = reader.read_le::<[u16; 3]>() {
            indices.push(TriangleIndices {
                _index0: triangle_indices[2],
                _index1: triangle_indices[1],
                _index2: triangle_indices[0],
            });
        }

        Ok(MOVI { _indices: indices })
    }
}

impl TypedFileChunk for MOVI {
    fn name(&self) -> &str {
        "MOVI"
    }
}

pub struct MOVT {
    _vertices: Vec<Vector3>,
}

impl MOVT {
    pub fn parse(raw: &Vec<u8>) -> Result<MOVT, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let mut vertices = Vec::new();
        while let Ok(triangle_vertices) = reader.read_le::<Vector3>() {
            vertices.push(triangle_vertices);
        }

        Ok(MOVT {
            _vertices: vertices,
        })
    }
}

impl TypedFileChunk for MOVT {
    fn name(&self) -> &str {
        "MOVT"
    }
}

pub struct MONR {
    _normals: Vec<Vector3>,
}

impl MONR {
    pub fn parse(raw: &Vec<u8>) -> Result<MONR, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let mut normals = Vec::new();
        while let Ok(triangle_vertices) = reader.read_le::<Vector3>() {
            normals.push(triangle_vertices);
        }

        Ok(MONR { _normals: normals })
    }
}

impl TypedFileChunk for MONR {
    fn name(&self) -> &str {
        "MONR"
    }
}

pub struct MOTV {
    _raw: Vec<u8>,
}

impl MOTV {
    pub fn parse(raw: &Vec<u8>) -> Result<MOTV, binrw::Error> {
        Ok(MOTV { _raw: raw.clone() })
    }
}

impl TypedFileChunk for MOTV {
    fn name(&self) -> &str {
        "MOTV"
    }
}

#[binread]
pub struct MOBA {
    _culling_bbox: BoundingBox,
    _start_index: u16,        // Offset into MOVI
    _index_count: u16,        // Number of MOVI indices used
    _vertex_start_index: u16, // Offset into MOVT
    _vertex_end_index: u16,   // Offset into MOVT, inclusive
    _unk: u8,
    _material_index: u8, // Offset into MOMT
}

impl MOBA {
    pub fn parse(raw: &Vec<u8>) -> Result<MOBA, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let moba = reader.read_le()?;

        Ok(moba)
    }
}

impl TypedFileChunk for MOBA {
    fn name(&self) -> &str {
        "MOBA"
    }
}

pub struct MOBN {
    _raw: Vec<u8>,
}

impl MOBN {
    pub fn parse(raw: &Vec<u8>) -> Result<MOBN, binrw::Error> {
        Ok(MOBN { _raw: raw.clone() })
    }
}

impl TypedFileChunk for MOBN {
    fn name(&self) -> &str {
        "MOBN"
    }
}

pub struct MOBR {
    _raw: Vec<u8>,
}

impl MOBR {
    pub fn parse(raw: &Vec<u8>) -> Result<MOBR, binrw::Error> {
        Ok(MOBR { _raw: raw.clone() })
    }
}

impl TypedFileChunk for MOBR {
    fn name(&self) -> &str {
        "MOBR"
    }
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum WMOGroupFlags {
    HasBSPNodes = 0x1,
    Flag0x2 = 0x2,
    HasVertexColors = 0x4,
    Outdoor = 0x8,
    Flag0x10 = 0x10,
    Flag0x20 = 0x20,
    Flag0x40 = 0x40,
    Flag0x80 = 0x80,
    Flag0x100 = 0x100,
    HasLights = 0x200,
    HasMPChunks = 0x400,
    HasDoodads = 0x800,
    HasWater = 0x1000,
    Indoor = 0x2000,
    Flag0x4000 = 0x4000,
    Flag0x8000 = 0x8000,
    Flag0x10000 = 0x10000,
    HasMORChunks = 0x20000,
    HasSkybox = 0x40000,
    Flag0x80000 = 0x80000,
    HasMOCV2 = 0x1000000,
    HasMOTV2 = 0x2000000,
}

#[binread]
pub struct MLIQ {
    _x_vertex_count: u32,
    _y_vertex_count: u32,
    _x_tile_count: u32,
    _y_tile_count: u32,
    _base_coordinates: Vector3,
    _material_id: u16,
}

impl MLIQ {
    pub fn parse(raw: &Vec<u8>) -> Result<MLIQ, binrw::Error> {
        let mut reader = Cursor::new(raw);
        let mliq: MLIQ = reader.read_le()?;

        Ok(mliq)
    }
}

impl TypedFileChunk for MLIQ {
    fn name(&self) -> &str {
        "MLIQ"
    }
}
