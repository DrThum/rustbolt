use binrw::binwrite;

pub mod object_guid;
pub mod update;
pub mod update_fields;

pub mod item;
pub mod player;

mod internal_values;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum ObjectTypeId {
    Object = 0,
    Item = 1,
    Container = 2,
    Unit = 3,
    Player = 4,
    GameObject = 5,
    DynamicObject = 6,
    Corpse = 7,
}

// FIXME: not sure this is the best place
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
