use std::f32::consts;

use binrw::binrw;
use shared::models::terrain_info::Vector3;
use shipyard::Component;

use crate::game::map_manager::MapKey;

#[binrw]
#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
}

impl Position {
    pub fn is_same_spot(&self, other: &Position) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }

    pub fn vec3(&self) -> Vector3 {
        Vector3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }

    pub fn distance_to(&self, other: Position, is_3d: bool) -> f32 {
        let dist_x = self.x - other.x;
        let dist_y = self.y - other.y;

        if is_3d {
            let dist_z = self.z - other.z;

            (dist_x * dist_x + dist_y * dist_y + dist_z * dist_z).sqrt()
        } else {
            (dist_x * dist_x + dist_y * dist_y).sqrt()
        }
    }

    pub fn center_between(&self, other: Position) -> Position {
        Position {
            x: (self.x + other.x) / 2.,
            y: (self.y + other.y) / 2.,
            z: (self.z + other.z) / 2.,
            o: 0.,
        }
    }
}

#[derive(Copy, Clone, Component, Debug, PartialEq)]
pub struct WorldPosition {
    pub map_key: MapKey,
    pub zone: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
}

impl WorldPosition {
    pub fn as_position(&self) -> Position {
        Position {
            x: self.x,
            y: self.y,
            z: self.z,
            o: self.o,
        }
    }

    pub fn vec3(&self) -> Vector3 {
        Vector3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }

    pub fn distance_to(&self, other: &WorldPosition, is_3d: bool) -> f32 {
        if self.map_key != other.map_key {
            panic!("measuring distance from WorldPositions on different maps");
        }

        let dist_x = self.x - other.x;
        let dist_y = self.y - other.y;

        if is_3d {
            let dist_z = self.z - other.z;

            (dist_x * dist_x + dist_y * dist_y + dist_z * dist_z).sqrt()
        } else {
            (dist_x * dist_x + dist_y * dist_y).sqrt()
        }
    }

    pub fn update_local(&mut self, pos: &Position) {
        self.x = pos.x;
        self.y = pos.y;
        self.z = pos.z;
        self.o = pos.o;
    }

    pub fn get_2d_angle_with(&self, other: &WorldPosition) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;

        let angle = f32::atan2(dy, dx);
        if angle >= 0. {
            angle
        } else {
            angle + 2. * consts::PI
        }
    }
}
