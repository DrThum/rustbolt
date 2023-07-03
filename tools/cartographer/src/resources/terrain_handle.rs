use bevy::{
    prelude::{Handle, Resource},
    utils::HashMap,
};

use crate::models::terrain::WrappedTerrainBlock;

#[derive(Resource, Default, Debug)]
pub struct TerrainHandle(pub HashMap<Handle<WrappedTerrainBlock>, (i32, i32)>);
