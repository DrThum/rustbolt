use binrw::binrw;

#[binrw]
#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
}

#[derive(Copy, Clone)]
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

    pub fn distance_to(&self, other: &WorldPosition, is_3d: bool) -> f32 {
        if self.map != other.map {
            panic!("measuring distance from WorldPositions on different maps");
        }

        let dist_x = self.x - other.x;
        let dist_y = self.y - other.y;

        if is_3d {
            let dist_z = self.z - other.z;

            dist_x * dist_x + dist_y * dist_y + dist_z * dist_z
        } else {
            dist_x * dist_x + dist_y * dist_y
        }
    }
}
