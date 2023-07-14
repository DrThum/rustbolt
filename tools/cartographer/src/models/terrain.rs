use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    reflect::impl_type_uuid,
    utils::BoxedFuture,
};
use binrw::{binread, io::Cursor, BinReaderExt};
use shared::models::{terrain_info::TerrainBlock, wmo::WmoMesh};

#[derive(Debug)]
#[binread]
pub struct WrappedTerrainBlock {
    pub terrain: TerrainBlock,
    _wmo_count: u32,
    #[br(count = _wmo_count)]
    pub wmo_meshes: Vec<WmoMesh>,
}

impl_type_uuid!(WrappedTerrainBlock, "269b2e4a5af644e0833bd65e29f5342d");

#[derive(Default)]
pub struct TerrainBlockLoader;

impl AssetLoader for TerrainBlockLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            if bytes.len() > 0 {
                let mut reader = Cursor::new(bytes);
                let terrain_block: WrappedTerrainBlock = reader.read_le()?;
                load_context.set_default_asset(LoadedAsset::new(terrain_block));
                Ok(())
            } else {
                Err(anyhow::Error::msg("non-existing terrain file"))
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["terrain"]
    }
}

pub fn interpolate_height_map(height_map: &Vec<f32>) -> Vec<f32> {
    assert!(height_map.len() == 145);
    let mut interpolated_nested: Vec<Vec<f32>> = Vec::new();

    for (idx, &height) in height_map.iter().enumerate() {
        if (idx + 17 - 8) % 17 == 0 {
            // End of outer verticex row (8, 25, 42, ...): nothing to interpolate
            interpolated_nested.push(vec![height]);
        } else if idx % 17 < 8 {
            // Outer vertices row (0-7, 17-24, ...): interpolate the mean between the current point and the next one
            let mean = (height + height_map[idx + 1]) / 2.0;
            interpolated_nested.push(vec![height, mean]);
        } else {
            // Inner vertices (9-16, 26-33, ...)
            if idx % 17 == 9 {
                // First inner vertex of the row, interpolate before and after
                interpolated_nested.push(vec![
                    (height_map[idx - 9] + height_map[idx + 8]) / 2.0,
                    height,
                    (height_map[idx - 8] + height_map[idx + 9]) / 2.0,
                ]);
            } else {
                // Other vertices, only interpolate after
                interpolated_nested.push(vec![
                    height,
                    (height_map[idx - 8] + height_map[idx + 9]) / 2.0,
                ]);
            }
        }
    }

    let flattened: Vec<f32> = interpolated_nested.into_iter().flatten().collect();
    assert!(flattened.len() == 17 * 17);
    flattened
}

// https://observablehq.com/@mourner/martin-real-time-rtin-terrain-mesh
// https://github.com/mapbox/martini/blob/main/index.js
pub struct Rtin {
    pub grid_size: usize,
    pub tile_size: f32,
    pub num_triangles: usize,
    pub num_parent_triangles: usize,
    pub coords: Vec<usize>,
}

impl Rtin {
    pub fn new(grid_width: usize, tile_size: f32) -> Self {
        assert!(
            Self::is_power_of_two(grid_width - 1),
            "grid width must be 2^k+1"
        );

        let tile_count_width = grid_width - 1;
        let num_triangles = tile_count_width * tile_count_width * 2 - 2;
        let num_parent_triangles = num_triangles - (tile_count_width * tile_count_width);

        // Mapping from triangle coordinates to its index in an implicit binary tree
        let mut coords: Vec<usize> = vec![0; num_triangles * 4];
        for i in 0..num_triangles {
            let mut id = i + 2;
            let mut ax = 0;
            let mut ay = 0;
            let mut bx = 0;
            let mut by = 0;
            let mut cx = 0;
            let mut cy = 0;

            if id % 2 != 0 {
                // Bottom-left triangle
                bx = tile_count_width;
                by = tile_count_width;
                cx = tile_count_width;
            } else {
                // Top-right triangle
                ax = tile_count_width;
                ay = tile_count_width;
                cy = tile_count_width;
            }

            id = id / 2;
            while id > 1 {
                let mx = (ax + bx) / 2;
                let my = (ay + by) / 2;

                if id % 2 != 0 {
                    bx = ax;
                    by = ay;
                    ax = cx;
                    ay = cy;
                } else {
                    ax = bx;
                    ay = by;
                    bx = cx;
                    by = cy;
                }

                cx = mx;
                cy = my;

                id = id / 2;
            }

            let k = i * 4;

            coords[k] = ax;
            coords[k + 1] = ay;
            coords[k + 2] = bx;
            coords[k + 3] = by;
        }

        Self {
            grid_size: grid_width,
            tile_size,
            num_triangles,
            num_parent_triangles,
            coords,
        }
    }

    fn is_power_of_two(num: usize) -> bool {
        num != 0 && num & (num - 1) == 0
    }
}

pub struct Tile<'a> {
    rtin: &'a Rtin,
    terrain: &'a Vec<[f32; 3]>,
    errors: Vec<f32>,
}

impl<'a> Tile<'a> {
    pub fn new(terrain: &'a Vec<[f32; 3]>, rtin: &'a Rtin) -> Self {
        assert!(
            terrain.len() == rtin.grid_size * rtin.grid_size,
            "terrain must have length equal to rtin.grid_size ^ 2"
        );

        let mut tile = Self {
            rtin,
            terrain,
            errors: vec![0.0; terrain.len()],
        };

        tile.update();

        tile
    }

    fn update(&mut self) {
        for i in (0..self.rtin.num_triangles).rev() {
            let k = i * 4;
            let ax = self.rtin.coords[k];
            let ay = self.rtin.coords[k + 1];
            let bx = self.rtin.coords[k + 2];
            let by = self.rtin.coords[k + 3];
            let mx = (ax + bx) / 2;
            let my = (ay + by) / 2;
            let cx = mx + my - ay;
            let cy = my + ax - mx;

            // Calculate the error in the middle of the long edge of the triangle
            let size = self.rtin.grid_size;
            let interpolated_height = f32::floor(
                (self.terrain[ay * size + ax][1] + self.terrain[by * size + bx][1]) / 2.0,
            );
            let middle_index = my * size + mx;
            let middle_error = f32::abs(interpolated_height - self.terrain[middle_index][1]);

            self.errors[middle_index] = self.errors[middle_index].max(middle_error);

            if i < self.rtin.num_parent_triangles {
                // Accumulate with children for bigger triangles
                let left_child_index = ((ay + cy) / 2) * size + ((ax + cx) / 2);
                let right_child_index = ((by + cy) / 2) * size + ((bx + cx) / 2);

                self.errors[middle_index] = self.errors[middle_index]
                    .max(self.errors[left_child_index])
                    .max(self.errors[right_child_index]);
            }
        }
    }

    pub fn get_mesh(
        &self,
        max_error: f32,
        offset_x: f32,
        offset_z: f32,
        scale_factor: f32,
    ) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut num_vertices: usize = 0;
        let mut num_triangles = 0;
        let max = self.rtin.grid_size - 1;

        let mut indices: Vec<usize> = vec![0; self.rtin.grid_size * self.rtin.grid_size];

        fn count_elements(
            ax: f32,
            ay: f32,
            bx: f32,
            by: f32,
            cx: f32,
            cy: f32,
            errors: &Vec<f32>,
            max_error: f32,
            size: usize,
            indices: &mut Vec<usize>,
            num_vertices: &mut usize,
            num_triangles: &mut usize,
        ) {
            let mx = f32::floor((ax + bx) / 2.0) as usize;
            let my = f32::floor((ay + by) / 2.0) as usize;

            if f32::abs(ax - cx) + f32::abs(ay - cy) > 1.0 && errors[my * size + mx] > max_error {
                count_elements(
                    cx,
                    cy,
                    ax,
                    ay,
                    mx as f32,
                    my as f32,
                    errors,
                    max_error,
                    size,
                    indices,
                    num_vertices,
                    num_triangles,
                );
                count_elements(
                    bx,
                    by,
                    cx,
                    cy,
                    mx as f32,
                    my as f32,
                    errors,
                    max_error,
                    size,
                    indices,
                    num_vertices,
                    num_triangles,
                );
            } else {
                if indices[ay as usize * size + ax as usize] == 0 {
                    *num_vertices += 1;
                    indices[ay as usize * size + ax as usize] = *num_vertices;
                }

                if indices[by as usize * size + bx as usize] == 0 {
                    *num_vertices += 1;
                    indices[by as usize * size + bx as usize] = *num_vertices;
                }

                if indices[cy as usize * size + cx as usize] == 0 {
                    *num_vertices += 1;
                    indices[cy as usize * size + cx as usize] = *num_vertices;
                }

                *num_triangles += 1;
            }
        }

        count_elements(
            0.0,
            0.0,
            max as f32,
            max as f32,
            max as f32,
            0.0,
            &self.errors,
            max_error,
            self.rtin.grid_size,
            &mut indices,
            &mut num_vertices,
            &mut num_triangles,
        );
        count_elements(
            max as f32,
            max as f32,
            0.0,
            0.0,
            0.0,
            max as f32,
            &self.errors,
            max_error,
            self.rtin.grid_size,
            &mut indices,
            &mut num_vertices,
            &mut num_triangles,
        );

        let mut vertices: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; num_vertices];
        let mut triangles: Vec<u32> = vec![0; num_triangles * 3];

        let mut tri_index: usize = 0;

        fn process_triangle(
            ax: f32,
            ay: f32,
            bx: f32,
            by: f32,
            cx: f32,
            cy: f32,
            errors: &Vec<f32>,
            max_error: f32,
            size: usize,
            indices: &Vec<usize>,
            vertices: &mut Vec<[f32; 3]>,
            triangles: &mut Vec<u32>,
            tri_index: &mut usize,
            terrain: &Vec<[f32; 3]>,
            offset_x: f32,
            offset_z: f32,
            tile_size: f32,
            scale_factor: f32,
        ) {
            let mx = f32::floor((ax + bx) / 2.0);
            let my = f32::floor((ay + by) / 2.0);

            if f32::abs(ax - cx) + f32::abs(ay - cy) > 1.0
                && errors[my as usize * size + mx as usize] > max_error
            {
                // Triangle doesn't approximate the surface well enough; drill down further
                process_triangle(
                    cx,
                    cy,
                    ax,
                    ay,
                    mx,
                    my,
                    errors,
                    max_error,
                    size,
                    indices,
                    vertices,
                    triangles,
                    tri_index,
                    terrain,
                    offset_x,
                    offset_z,
                    tile_size,
                    scale_factor,
                );
                process_triangle(
                    bx,
                    by,
                    cx,
                    cy,
                    mx,
                    my,
                    errors,
                    max_error,
                    size,
                    indices,
                    vertices,
                    triangles,
                    tri_index,
                    terrain,
                    offset_x,
                    offset_z,
                    tile_size,
                    scale_factor,
                );
            } else {
                // Add a triangle
                let a = indices[ay as usize * size + ax as usize] - 1;
                let b = indices[by as usize * size + bx as usize] - 1;
                let c = indices[cy as usize * size + cx as usize] - 1;

                vertices[a][0] = (ax * tile_size + offset_x) * scale_factor;
                vertices[a][1] = terrain[ay as usize * size + ax as usize][1] * scale_factor;
                vertices[a][2] = (ay * tile_size + offset_z) * scale_factor;

                vertices[b][0] = (bx * tile_size + offset_x) * scale_factor;
                vertices[b][1] = terrain[by as usize * size + bx as usize][1] * scale_factor;
                vertices[b][2] = (by * tile_size + offset_z) * scale_factor;

                vertices[c][0] = (cx * tile_size + offset_x) * scale_factor;
                vertices[c][1] = terrain[cy as usize * size + cx as usize][1] * scale_factor;
                vertices[c][2] = (cy * tile_size + offset_z) * scale_factor;

                triangles[*tri_index] = a as u32;
                *tri_index += 1;

                triangles[*tri_index] = b as u32;
                *tri_index += 1;

                triangles[*tri_index] = c as u32;
                *tri_index += 1;
            }
        }

        process_triangle(
            0.0,
            0.0,
            max as f32,
            max as f32,
            max as f32,
            0.0,
            &self.errors,
            max_error,
            self.rtin.grid_size,
            &mut indices,
            &mut vertices,
            &mut triangles,
            &mut tri_index,
            &self.terrain,
            offset_x,
            offset_z,
            self.rtin.tile_size,
            scale_factor,
        );

        process_triangle(
            max as f32,
            max as f32,
            0.0,
            0.0,
            0.0,
            max as f32,
            &self.errors,
            max_error,
            self.rtin.grid_size,
            &mut indices,
            &mut vertices,
            &mut triangles,
            &mut tri_index,
            &self.terrain,
            offset_x,
            offset_z,
            self.rtin.tile_size,
            scale_factor,
        );

        (vertices, triangles)
    }
}
