use binrw::binwrite;

#[binwrite]
#[derive(Copy, Clone)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
}

pub struct WorldPosition {
    pub map: u32,
    pub zone: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
}
