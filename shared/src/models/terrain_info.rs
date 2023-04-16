use binrw::binrw;
use enumflags2::{bitflags, BitFlags};
use enumn::N;
use fixedbitset::FixedBitSet;

pub const TERRAIN_INFO_MAGIC: [u8; 4] = [b'T', b'E', b'R', b'R'];
pub const TERRAIN_INFO_VERSION: u32 = 1;

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
 * If there is a hole, then height is INVALID_HEIGHT, otherwise use the height map
 */

#[binrw]
#[allow(dead_code)]
pub struct TerrainInfo {
    magic: [u8; 4],
    version: u32,
    #[br(count = 256)]
    chunks: Vec<TerrainChunk>,
}

impl TerrainInfo {
    pub fn new(chunks: Vec<TerrainChunk>) -> Self {
        assert!(chunks.len() == 256, "TerrainInfo expects 256 TerrainChunks");

        Self {
            magic: TERRAIN_INFO_MAGIC,
            version: TERRAIN_INFO_VERSION,
            chunks,
        }
    }
}

#[binrw]
#[allow(dead_code)]
pub struct TerrainChunk {
    row: u32, // index_x in MCNK
    col: u32, // index_y in MCNK
    area_id: u32,
    base_height: f32,
    #[bw(map = |bs: &FixedBitSet| bs.as_slice()[0])]
    #[br(map = |bits: u32| FixedBitSet::with_capacity_and_blocks(16, vec![bits]))]
    holes: FixedBitSet, // See explanation on top of this file
    height_map: HeightMap, // See explanation on top of this file
    #[bw(map = |cond: &bool| if *cond { 1_u8 } else { 0_u8 })]
    #[br(map = |v: u8| v == 1)]
    has_liquid: bool,
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
}

#[binrw]
#[allow(dead_code)]
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
#[derive(Clone, Copy, N)]
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
#[derive(Clone, Copy)]
pub enum LiquidFlags {
    Water = 0x01,
    Ocean = 0x02,
    Magma = 0x04,
    Slime = 0x08,
    DarkWater = 0x10,
}
