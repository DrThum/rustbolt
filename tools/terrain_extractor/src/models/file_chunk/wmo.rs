use binrw::io::Cursor;
use shared::models::{
    terrain_info::{Vector3, WmoPlacement},
    wmo::{WmoGroupMesh, WmoMesh},
};

use super::{FileChunk, FileType, MVER};

pub use self::chunks::*;

pub mod chunks;

pub struct WMO {
    pub groups: Vec<WMOGroup>,
}

pub struct WMORoot {
    pub group_count: u32,
}

pub struct WMOGroup {
    pub movi: Box<MOVI>,
    pub movt: Box<MOVT>,
}

impl WMO {
    pub fn parse_root(raw: &Vec<u8>) -> Option<WMORoot> {
        let mut reader = Cursor::new(raw);
        let _mver: Box<MVER> = FileChunk::read_as(FileType::WMO, &mut reader);
        let mohd: Box<MOHD> = FileChunk::read_as(FileType::WMO, &mut reader);

        Some(WMORoot {
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
        let movi: Box<MOVI> = FileChunk::read_as(FileType::WMO, &mut reader);
        let movt: Box<MOVT> = FileChunk::read_as(FileType::WMO, &mut reader);
        let _monr: Box<MONR> = FileChunk::read_as(FileType::WMO, &mut reader);
        let _motv: Box<MOTV> = FileChunk::read_as(FileType::WMO, &mut reader);
        let _moba: Box<MOBA> = FileChunk::read_as(FileType::WMO, &mut reader);
        if mogp.flags.contains(WMOGroupFlags::HasLights) {
            let _molr: Box<MOLR> = FileChunk::read_as(FileType::WMO, &mut reader);
        }
        if mogp.flags.contains(WMOGroupFlags::HasDoodads) {
            let _modr: Box<MODR> = FileChunk::read_as(FileType::WMO, &mut reader);
        }
        if mogp.flags.contains(WMOGroupFlags::HasBSPNodes) {
            let _mobn: Box<MOBN> = FileChunk::read_as(FileType::WMO, &mut reader);
            let _mobr: Box<MOBR> = FileChunk::read_as(FileType::WMO, &mut reader);
        }
        if mogp.flags.contains(WMOGroupFlags::HasVertexColors) {
            let _mocv: Box<MOCV> = FileChunk::read_as(FileType::WMO, &mut reader);
        }
        if mogp.flags.contains(WMOGroupFlags::HasWater) {
            let _mliq: Box<MLIQ> = FileChunk::read_as(FileType::WMO, &mut reader);
        }
        // TODO: More optional chunks are possible here

        Some(WMOGroup { movi, movt })
    }

    pub fn export_mesh(&self, placement: &WmoPlacement) -> WmoMesh {
        let groups: Vec<WmoGroupMesh> = self
            .groups
            .iter()
            .map(|client_group| {
                let indices: Vec<[u16; 3]> = client_group
                    .movi
                    .as_ref()
                    .indices
                    .iter()
                    .map(|i| i.as_array())
                    .collect();

                let vertices: Vec<Vector3> = client_group
                    .movt
                    .as_ref()
                    .vertices
                    .clone()
                    .into_iter()
                    .map(|vertex| {
                        // Apply rotation
                        // FIXME: is there sometimes another rotation than around the vertical axis?
                        let vec3 = glm::vec3(vertex.x, vertex.y, vertex.z);
                        // Use rotation.y which is actually "rotation.b" and corresponds to the Z
                        // axis
                        let vec3_rotated =
                            glm::rotate_z_vec3(&vec3, (placement.rotation.y + 180.).to_radians());
                        Vector3 {
                            x: vec3_rotated.x,
                            y: vec3_rotated.y,
                            z: vec3_rotated.z,
                        }
                    })
                    .collect();

                WmoGroupMesh {
                    triangle_count: indices.len() as u32,
                    indices,
                    vertex_count: vertices.len() as u32,
                    vertices,
                }
            })
            .collect();

        WmoMesh {
            group_count: groups.len() as u32,
            groups,
            position: placement.position,
        }
    }
}
