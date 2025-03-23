use std::collections::HashMap;

use parry3d::{
    math::{Isometry, Point},
    query::{Ray, RayCast},
};
use shared::models::terrain_info::{Terrain, TerrainBlock, BLOCK_WIDTH, MAP_WIDTH_IN_BLOCKS};

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
}
