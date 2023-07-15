use binrw::io::Cursor;
use shared::models::terrain_info::{TerrainBlock, TerrainChunk, TerrainLiquidInfo, WmoPlacement};

pub use self::chunks::*;

use super::{FileChunk, FileType, TypedFileChunk, MVER};

pub mod chunks;

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
        let _mver: Box<MVER> = FileChunk::read_as(FileType::ADT, &mut reader);
        let _mhdr: Box<MHDR> = FileChunk::read_as(FileType::ADT, &mut reader);
        let _mcin: Box<MCIN> = FileChunk::read_as(FileType::ADT, &mut reader);
        let _mtex: Box<MTEX> = FileChunk::read_as(FileType::ADT, &mut reader);
        let _mmdx: Box<MMDX> = FileChunk::read_as(FileType::ADT, &mut reader);
        let _mmid: Box<MMID> = FileChunk::read_as(FileType::ADT, &mut reader);
        let mwmo: Box<MWMO> = FileChunk::read_as(FileType::ADT, &mut reader);
        let mwid: Box<MWID> = FileChunk::read_as(FileType::ADT, &mut reader);
        let _mddf: Box<MDDF> = FileChunk::read_as(FileType::ADT, &mut reader);
        let modf: Box<MODF> = FileChunk::read_as(FileType::ADT, &mut reader);

        let mut mcnk_chunks: Vec<MCNK> = Vec::new();
        for _i in 0..256 {
            let mcnk = FileChunk::read_as(FileType::ADT, &mut reader);
            mcnk_chunks.push(*mcnk);
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

    pub(crate) fn wmos_to_extract(&self) -> Vec<WmoPlacement> {
        self.modf_chunk
            .wmo_placements
            .iter()
            .map(|modf_rec| {
                let mwmo_offset = self.mwid_chunk.offsets[modf_rec.mwid_index as usize] as usize;
                let wmo_root_path = self.mwmo_chunk.filenames.get(&mwmo_offset).unwrap().clone();

                WmoPlacement {
                    wmo_root_path,
                    position: modf_rec.position,
                    rotation: modf_rec.rotation,
                }
            })
            .collect()
    }
}
