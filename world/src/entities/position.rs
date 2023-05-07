use binrw::binrw;

#[binrw]
#[derive(Copy, Clone, Debug)]
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

impl WorldPosition {
    pub fn to_position(&self) -> Position {
        Position {
            x: self.x,
            y: self.y,
            z: self.z,
            o: self.o,
        }
    }
}
