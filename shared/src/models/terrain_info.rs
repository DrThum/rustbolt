use std::{
    fs,
    io::Read,
    ops::{Add, Div, Mul, Sub},
};

use binrw::{binrw, io::Cursor, BinReaderExt};
use enumflags2::{bitflags, BitFlags};
use enumn::N;
use fixedbitset::FixedBitSet;
use parry3d::{math::Point, shape::TriMesh};
use splines::impl_Interpolate;

use crate::models::wmo::WmoMesh;

pub const TERRAIN_BLOCK_MAGIC: [u8; 4] = [b'T', b'E', b'R', b'R'];
pub const TERRAIN_BLOCK_VERSION: u32 = 2;

pub const MAP_WIDTH_IN_BLOCKS: usize = 64;
pub const BLOCK_WIDTH_IN_CHUNKS: usize = 16;
pub const MAP_WIDTH_IN_CHUNKS: usize = MAP_WIDTH_IN_BLOCKS * BLOCK_WIDTH_IN_CHUNKS;

pub const CHUNK_WIDTH: f32 = 33.333333;
pub const BLOCK_WIDTH: f32 = CHUNK_WIDTH * BLOCK_WIDTH_IN_CHUNKS as f32; // 533.333328
pub const MAP_WIDTH: f32 = BLOCK_WIDTH * MAP_WIDTH_IN_BLOCKS as f32; // 34133.332992
pub const MAP_MAX_COORD: f32 = MAP_WIDTH / 2.0; // 17066.666496

pub type HeightMap = [f32; 145];

/*
 * Axes:
 *
 *     ^ +X
 * +Y  |
 * <---|---> -Y
 *     |
 *     v -X
 *
 * Going up is +Z
 *
 * Maps are divided in 64 * 64 = 4096 blocks
 *  `- not all of them exist (WDT)
 *
 * Blocks are divided in 16 * 16 = 256 chunks
 *   `- all of them exist (ADT)
 *
 * 1 chunk width = 100 feet (33.3333 yards)
 * 1 block width = 1600 feet (533.33333 yards)
 * 1 map width = 102400 feet (34133.33312 yards)
 * Max X and Y coords: +- 17066.66656
 *
 * Height map: 9 * 9 outer vertexes interleaved with 8 * 8 inner vertexes
 * Each value is an offset relative to the base height of the chunk
 *
 * 0    1    2    3    4    5    6    7    8
 *   9    10   11   12   13   14   15   16
 * 17   18   19   20   21   22   23   24   25
 *   26   27   28   29   30   31   32   33
 * 34   35   36   37   38   39   40   41   42
 *   43   44   45   46   47   48   49   50
 * 51   52   53   54   55   56   57   58   59
 *   60   61   62   63   64   65   66   67
 * 68   69   70   71   72   73   74   75   76
 *   77   78   79   80   81   82   83   84
 * 85   86   87   88   89   90   91   92   93
 *   94   95   96   97   98   99   100  101
 * 102  103  104  105  106  107  108  109  110
 *   111  112  113  114  115  116  117  118
 * 119  120  121  122  123  124  125  126  127
 *   128  129  130  131  132  133  134  135
 * 136  137  138  139  140  141  142  143  144
 *
 * Holes: bit mask of 16 values:
 *  - if mask & 0x1, hole in the square
 *             0 ------ 2
 *             |        |
 *             |        |
 *            34--------36
 *  - if mask & 0x2, hole in the square
 *             2--------4
 *             |        |
 *             |        |
 *            36--------38
 *
 *  and so on...
 *
 *  - if mask & 0x8000, hole in the square
 *           108--------110
 *             |        |
 *             |        |
 *           142--------144
 *
 * If there is a hole, then height is None, otherwise use the height map
 */

#[binrw]
#[allow(dead_code)]
#[derive(Debug)]
// 64 * 64 = 4096 blocks per map
// Remember to increment TERRAIN_BLOCK_VERSION when changing this struct!
pub struct TerrainBlock {
    magic: [u8; 4],
    version: u32,
    #[br(count = 256)]
    pub chunks: Vec<TerrainChunk>,
}

impl TerrainBlock {
    pub fn new(chunks: Vec<TerrainChunk>) -> Self {
        assert!(chunks.len() == 256, "TerrainInfo expects 256 TerrainChunks");

        Self {
            magic: TERRAIN_BLOCK_MAGIC,
            version: TERRAIN_BLOCK_VERSION,
            chunks,
            // num_wmo_placements: wmo_placements.len() as u32,
            // wmo_placements,
        }
    }

    pub fn load_from_disk(
        data_dir: &String,
        map_name: &String,
        row: usize,
        col: usize,
    ) -> Option<Terrain> {
        let filename = format!("{}/terrain/{}_{}_{}.terrain", data_dir, map_name, row, col);
        if let Ok(mut f) = fs::File::open(&filename) {
            let metadata = fs::metadata(&filename).expect("unable to read terrain file metadata");
            let mut buffer = Vec::new();
            buffer.resize(metadata.len() as usize, 0);
            f.read(&mut buffer).unwrap();

            let mut reader = Cursor::new(buffer);
            let terrain_block: TerrainBlock = reader.read_le().unwrap();

            assert!(
                terrain_block.magic == TERRAIN_BLOCK_MAGIC,
                "{} is not a valid terrain file",
                filename
            );
            assert!(
                terrain_block.version == TERRAIN_BLOCK_VERSION,
                "{} is outdated, please re-extract terrain files",
                filename
            );

            let wmo_count: u32 = reader.read_le().unwrap();
            let mut vertices: Vec<Point<f32>> = Vec::new();
            let mut indices: Vec<[u32; 3]> = Vec::new();
            for _ in 0..wmo_count {
                let mesh: WmoMesh = reader.read_le().unwrap();

                for group in mesh.groups.iter() {
                    let base_index = vertices.len() as u32;
                    let mut this_group_vertices: Vec<Point<f32>> = group
                        .vertices
                        .iter()
                        .map(|vertex| {
                            Point::new(
                                vertex.x + mesh.position.x,
                                vertex.y + mesh.position.y,
                                vertex.z + mesh.position.z,
                            )
                        })
                        .collect();
                    vertices.append(&mut this_group_vertices);

                    let mut this_group_indices: Vec<[u32; 3]> = group
                        .indices
                        .iter()
                        .map(|t| t.map(|i| base_index + i as u32))
                        .collect();
                    indices.append(&mut this_group_indices);
                }
            }

            let collision_mesh = if wmo_count > 0 {
                Some(TriMesh::new(vertices, indices))
            } else {
                None
            };

            Some(Terrain {
                ground: terrain_block,
                collision_mesh,
            })
        } else {
            None
        }
    }

    pub fn get_height(&self, position_x: f32, position_y: f32) -> Option<f32> {
        let chunk_row =
            ((512.0 - (position_x / CHUNK_WIDTH)) % BLOCK_WIDTH_IN_CHUNKS as f32).floor() as usize;
        let chunk_col =
            ((512.0 - (position_y / CHUNK_WIDTH)) % BLOCK_WIDTH_IN_CHUNKS as f32).floor() as usize;

        let chunk = self.get_chunk(chunk_row, chunk_col);

        let subchunk_width = CHUNK_WIDTH / 8.0;
        let x_offset_in_chunk = (MAP_MAX_COORD - position_x) % CHUNK_WIDTH;
        let y_offset_in_chunk = (MAP_MAX_COORD - position_y) % CHUNK_WIDTH;

        if chunk.has_hole_at(x_offset_in_chunk, y_offset_in_chunk) {
            return None;
        }

        let x_offset_in_chunk = x_offset_in_chunk / subchunk_width;
        let y_offset_in_chunk = y_offset_in_chunk / subchunk_width;

        let row_start_index = x_offset_in_chunk.floor() as usize; // Outer vertex
        let row_end_index = row_start_index + 1;

        let col_start_index = y_offset_in_chunk.floor() as usize; // Outer vertex
        let col_end_index = col_start_index + 1;

        // +--------------> Y offset
        // | tl-------tr
        // | | \  1  / |
        // | |  \   /  |
        // | | 2  c  3 |
        // | |  /   \  |
        // | | /  4  \ |
        // | bl-------br
        // V X offset
        //
        // First, calculate which triangle the point is in
        // Then solve an equation depending on the triangle

        let top_left = chunk.height_map[row_start_index * 17 + col_start_index];
        let top_right = chunk.height_map[row_start_index * 17 + col_end_index];
        let center = chunk.height_map[row_start_index * 17 + col_start_index + 9];
        let bottom_left = chunk.height_map[row_end_index * 17 + col_start_index];
        let bottom_right = chunk.height_map[row_end_index * 17 + col_end_index];

        let normalized_chunk_offset_x = x_offset_in_chunk / CHUNK_WIDTH;
        let normalized_chunk_offset_y = y_offset_in_chunk / CHUNK_WIDTH;

        let height;
        if x_offset_in_chunk + y_offset_in_chunk < 1.0 {
            if x_offset_in_chunk < y_offset_in_chunk {
                // Triangle 1
                height = top_left
                    + (top_right - top_left) * normalized_chunk_offset_y
                    + (center - top_left) * (2.0 * normalized_chunk_offset_x);
            } else {
                // Triangle 2
                height = top_left
                    + (bottom_left - top_left) * normalized_chunk_offset_x
                    + (center - top_left) * (2.0 * normalized_chunk_offset_y);
            }
        } else {
            if x_offset_in_chunk < y_offset_in_chunk {
                // Triangle 3
                height = top_right
                    + (bottom_right - top_right) * normalized_chunk_offset_x
                    + (center - top_right) * 2.0 * (1.0 - normalized_chunk_offset_y);
            } else {
                // Triangle 4
                height = bottom_left
                    + (bottom_right - bottom_left) * normalized_chunk_offset_y
                    + (center - bottom_left) * (2.0 * (1.0 - normalized_chunk_offset_x));
            }
        }

        Some(height + chunk.base_height)
    }

    fn get_chunk(&self, row: usize, col: usize) -> &TerrainChunk {
        // FIXME: Access by index instead for O(1) instead of O(n)
        self.chunks
            .iter()
            .find(|chunk| chunk.row as usize == row && chunk.col as usize == col)
            .unwrap()
    }
}

#[binrw]
#[allow(dead_code)]
#[derive(Debug)]
// 16 * 16 = 256 chunks per block
pub struct TerrainChunk {
    row: u32, // index_x in MCNK
    col: u32, // index_y in MCNK
    area_id: u32,
    pub base_height: f32,
    #[bw(map = |bs: &FixedBitSet| bs.as_slice()[0])]
    #[br(map = |bits: u32| FixedBitSet::with_capacity_and_blocks(16, vec![bits]))]
    holes: FixedBitSet, // See explanation on top of this file
    pub height_map: HeightMap, // See explanation on top of this file
    #[bw(map = |cond: &bool| if *cond { 1_u8 } else { 0_u8 })]
    #[br(map = |v: u8| v == 1)]
    has_liquid: bool,
    #[br(if(has_liquid))]
    liquid_info: Option<TerrainLiquidInfo>,
}

impl TerrainChunk {
    pub fn new(
        row: u32,
        col: u32,
        area_id: u32,
        base_height: f32,
        holes: u32,
        height_map: HeightMap,
        liquid_info: Option<TerrainLiquidInfo>,
    ) -> Self {
        // Holes are stored as u32 in the client but only the 16 least significant bytes are used
        let holes = FixedBitSet::with_capacity_and_blocks(16, vec![holes & 0xFFFF]);

        Self {
            row,
            col,
            area_id,
            base_height,
            holes,
            height_map,
            has_liquid: liquid_info.is_some(),
            liquid_info,
        }
    }

    pub fn has_hole_at(&self, x_offset_in_chunk: f32, y_offset_in_chunk: f32) -> bool {
        // Find which square contains the coordinates
        let hole_width = CHUNK_WIDTH / 4.0;
        let row = (x_offset_in_chunk / hole_width).floor() as usize;
        let col = (y_offset_in_chunk / hole_width).floor() as usize;

        // Calculate the bit index in self.holes
        let bit_index = row * 4 + col;

        // Return whether that bit is set
        self.holes.contains(bit_index)
    }
}

#[derive(Debug)]
pub struct WmoPlacement {
    pub wmo_root_path: String,
    pub position: Vector3,
    pub rotation: Vector3,
}

// TODO: Move this somewhere else
#[binrw]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const ZERO: Self = Self {
        x: 0.,
        y: 0.,
        z: 0.,
    };

    pub fn as_array(&self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    pub fn add(&self, other: &Vector3) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    pub fn sub(&self, other: &Vector3) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    pub fn div(&self, factor: f32) -> Self {
        Self {
            x: self.x / factor,
            y: self.y / factor,
            z: self.z / factor,
        }
    }

    pub fn len(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn lerp(&self, other: &Vector3, t: f32) -> Vector3 {
        *self + (*other - *self) * t
    }

    // Blizz magic
    pub fn pack(&self) -> u32 {
        let mut packed: u32 = 0;
        packed |= ((self.x / 0.25) as i32 & 0x7FF) as u32;
        packed |= (((self.y / 0.25) as i32 & 0x7FF) << 11) as u32;
        packed |= (((self.z / 0.25) as i32 & 0x3FF) << 22) as u32;
        packed
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Sub<f32> for Vector3 {
    type Output = Self;

    fn sub(self, factor: f32) -> Self::Output {
        Self {
            x: self.x - factor,
            y: self.y - factor,
            z: self.z - factor,
        }
    }
}

impl Mul for Vector3 {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

impl Mul<f32> for Vector3 {
    type Output = Self;

    fn mul(self, factor: f32) -> Self::Output {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }
}

impl Div for Vector3 {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z,
        }
    }
}

impl Div<f32> for Vector3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl_Interpolate!(f32, Vector3, std::f32::consts::PI);

#[binrw]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Vector3,
    pub max: Vector3,
}

#[binrw]
#[allow(dead_code)]
#[derive(Debug)]
pub struct TerrainLiquidInfo {
    entry: LiquidTypeEntry,
    #[br(map = |raw: u8| unsafe { BitFlags::from_bits_unchecked(raw) })]
    #[bw(map = |flags: &BitFlags<LiquidFlags>| flags.bits())]
    flags: BitFlags<LiquidFlags>,
    height_map: [f32; 9 * 9],
    #[br(map = |raw: [u8; 8*8]| raw.map(|v| v == 1))]
    #[bw(map = |bools: &[bool; 8*8]| bools.map(|v| if v { 1_u8 } else { 0_u8 }))]
    liquid_map: [bool; 8 * 8], // Based on DontRender (0xF) flag in the client data
}

impl TerrainLiquidInfo {
    pub fn new(
        entry: LiquidTypeEntry,
        flags: BitFlags<LiquidFlags>,
        height_map: [f32; 9 * 9],
        liquid_map: [bool; 8 * 8],
    ) -> Self {
        Self {
            entry,
            flags,
            height_map,
            liquid_map,
        }
    }
}

#[binrw]
#[br(repr = u8)]
#[bw(repr = u8)]
#[derive(Clone, Copy, N, Debug)]
pub enum LiquidTypeEntry {
    NoWater = 0,
    Water = 1,
    Ocean = 2,
    Magma = 3,
    Slime = 4,
}

#[allow(dead_code)]
#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum LiquidFlags {
    Water = 0x01,
    Ocean = 0x02,
    Magma = 0x04,
    Slime = 0x08,
    DarkWater = 0x10,
}

pub struct Terrain {
    pub ground: TerrainBlock,
    pub collision_mesh: Option<TriMesh>,
}
