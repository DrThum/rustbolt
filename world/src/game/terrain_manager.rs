use std::collections::HashMap;

use parry3d::{
    math::{Isometry, Point},
    query::{Ray, RayCast},
};
use rand::Rng;
use shared::models::terrain_info::{
    Terrain, TerrainBlock, Vector3, BLOCK_WIDTH, MAP_WIDTH_IN_BLOCKS,
};

use crate::{create_wrapped_resource, entities::position::WorldPosition};

use super::map_manager::TerrainBlockCoords;

// Add a safety margin when calculating ground height to account for rounding when generating
// terrain data and ensure that our ray (when we are within a WMO) starts from high enough to
// actually hit the floor.
pub const HEIGHT_MEASUREMENT_TOLERANCE: f32 = 1.;

pub struct TerrainManager {
    blocks: HashMap<TerrainBlockCoords, Terrain>,
}

impl TerrainManager {
    pub fn load(data_dir: &String, map_internal_name: &String) -> Self {
        let mut map_terrains: HashMap<TerrainBlockCoords, Terrain> = HashMap::new();

        for row in 0..MAP_WIDTH_IN_BLOCKS {
            for col in 0..MAP_WIDTH_IN_BLOCKS {
                let maybe_terrain =
                    TerrainBlock::load_from_disk(&data_dir, &map_internal_name, row, col);

                if let Some(terrain) = maybe_terrain {
                    let key = TerrainBlockCoords { row, col };
                    map_terrains.insert(key, terrain);
                }
            }
        }

        Self {
            blocks: map_terrains,
        }
    }

    pub fn empty() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }

    pub fn get_ground_or_floor_height(
        &self,
        position_x: f32,
        position_y: f32,
        position_z: f32,
    ) -> Option<f32> {
        let offset: f32 = MAP_WIDTH_IN_BLOCKS as f32 / 2.0;
        let block_row = (offset - (position_x / BLOCK_WIDTH)).floor() as usize;
        let block_col = (offset - (position_y / BLOCK_WIDTH)).floor() as usize;
        let terrain_block_coords = TerrainBlockCoords {
            row: block_row,
            col: block_col,
        };

        self.blocks.get(&terrain_block_coords).and_then(|terrain| {
            // Check if we are within a WMO first
            let ray = Ray::new(
                Point::new(
                    position_x,
                    position_y,
                    position_z + HEIGHT_MEASUREMENT_TOLERANCE,
                ),
                -parry3d::na::Vector3::z(),
            );
            let time_of_impact = terrain
                .collision_mesh
                .as_ref()
                .and_then(|mesh| mesh.cast_ray(&Isometry::identity(), &ray, f32::MAX, false));

            let intersection_point = time_of_impact.map(|toi| ray.origin + ray.dir * toi);
            let wmo_height = intersection_point
                .map(|ip| ip[2])
                .filter(|&h| h <= (position_z + HEIGHT_MEASUREMENT_TOLERANCE));

            // Fallback on the ground
            let ground_height = terrain
                .ground
                .get_height(position_x, position_y)
                .filter(|&h| h <= (position_z + HEIGHT_MEASUREMENT_TOLERANCE));

            match (wmo_height, ground_height) {
                (None, None) => None,
                (None, Some(_)) => ground_height,
                (Some(_), None) => wmo_height,
                (Some(wmo), Some(ground)) => Some(wmo.max(ground)),
            }
        })
    }

    pub fn get_area_id(&self, position_x: f32, position_y: f32) -> Option<u32> {
        let offset: f32 = MAP_WIDTH_IN_BLOCKS as f32 / 2.0;
        let block_row = (offset - (position_x / BLOCK_WIDTH)).floor() as usize;
        let block_col = (offset - (position_y / BLOCK_WIDTH)).floor() as usize;
        let terrain_block_coords = TerrainBlockCoords {
            row: block_row,
            col: block_col,
        };

        self.blocks.get(&terrain_block_coords).map(|terrain| {
            // TODO: Area ID might come from the WMO
            terrain.ground.get_area_id(position_x, position_y)
        })
    }

    pub fn get_random_point_around(&self, origin: &Vector3, radius: f32) -> Vector3 {
        if radius <= 0. {
            return *origin;
        }

        let mut rng = rand::thread_rng();
        let angle: f32 = rng.gen_range(0.0..2. * std::f32::consts::PI);
        let distance = rng.gen_range(0.0..=radius);

        let random_x = origin.x + distance * angle.cos();
        let random_y = origin.y + distance * angle.sin();
        let z = self
            .get_ground_or_floor_height(random_x, random_y, origin.z)
            .unwrap_or(origin.z);

        Vector3::new(random_x, random_y, z)
    }

    pub fn get_point_around_at_angle(
        &self,
        origin: &WorldPosition,
        distance: f32,
        angle: f32,
    ) -> WorldPosition {
        let x = origin.x + distance * angle.cos();
        let y = origin.y + distance * angle.sin();

        let z = self
            .get_ground_or_floor_height(x, y, origin.z)
            .unwrap_or(origin.z);

        let mut point = *origin;
        point.x = x;
        point.y = y;
        point.z = z;

        point
    }
}

create_wrapped_resource!(WrappedTerrainManager, TerrainManager);
