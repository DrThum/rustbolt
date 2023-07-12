use binrw::binrw;

use super::terrain_info::Vector3;

#[binrw]
#[derive(Debug)]
pub struct WmoMesh {
    pub position: Vector3,
    pub group_count: u32,
    #[br(count = group_count)]
    pub groups: Vec<WmoGroupMesh>,
}

#[binrw]
#[derive(Debug)]
pub struct WmoGroupMesh {
    pub triangle_count: u32,
    #[br(count = triangle_count)]
    pub indices: Vec<[u16; 3]>,
    pub vertex_count: u32,
    #[br(count = vertex_count)]
    pub vertices: Vec<Vector3>,
}
