use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use models::terrain::{self, Rtin, Tile, WrappedTerrainBlock};
use resources::terrain_handle::TerrainHandle;
use shared::models::terrain_info::{
    Vector3, BLOCK_WIDTH, BLOCK_WIDTH_IN_CHUNKS, CHUNK_WIDTH, MAP_WIDTH_IN_BLOCKS,
};
use smooth_bevy_cameras::controllers::orbit::{OrbitCameraBundle, OrbitCameraController};

pub mod models {
    pub mod terrain;
}

pub mod resources {
    pub mod terrain_handle;
}

pub fn setup(
    mut commands: Commands,
    mut ambient_light: ResMut<AmbientLight>,
    server: Res<AssetServer>,
    mut terrain_handles: ResMut<TerrainHandle>,
) {
    for row in 31..33 {
        for col in 31..33 {
            let handle: Handle<WrappedTerrainBlock> =
                server.load(format!("data/terrain/Azeroth_{row}_{col}.terrain"));
            terrain_handles.0.insert(handle, (row, col));
        }
    }

    ambient_light.color = Color::WHITE;

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 32000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-20.0, 20.0, 0.0)
            .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Z),
        ..default()
    });

    commands
        .spawn(Camera3dBundle::default())
        .insert(OrbitCameraBundle::new(
            OrbitCameraController {
                mouse_rotate_sensitivity: Vec2::splat(0.5),
                mouse_translate_sensitivity: Vec2::splat(0.7),
                ..default()
            },
            Vec3::new(0.0, 1.2, 5.),
            Vec3::new(0., 1., 0.),
            Vec3::Y,
        ));
}

pub const SCALE_FACTOR: f32 = 0.01;
pub const HEIGHT_MAP_WIDTH: usize = 17;
pub const GRID_WIDTH: usize = 257;
pub const RTIN_SCALE_FACTOR: f32 = 2.0835;

// X axis is growing from left to right
// Y axis is growing towards the sky
// Z axis is growing from far away towards the camera
pub fn display_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ev_asset: EventReader<AssetEvent<WrappedTerrainBlock>>,
    terrains: Res<Assets<WrappedTerrainBlock>>,
    terrain_handles: Res<TerrainHandle>,
) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } => {
                let asset = &terrains.get(handle).unwrap();
                let terrain = &asset.terrain;
                let (block_row, block_col) = terrain_handles.0.get(handle).unwrap();
                let block_x_offset =
                    BLOCK_WIDTH * (block_col - (MAP_WIDTH_IN_BLOCKS as i32 / 2)) as f32;
                let block_z_offset =
                    BLOCK_WIDTH * (block_row - (MAP_WIDTH_IN_BLOCKS as i32 / 2)) as f32;

                // Each block is 16 chunks wide and each chunk has a height map of width 17.
                // But there is an overlap between the last row/column on a chunk and the first
                // row/column of the next chunk (except for the last one). So the total width
                // (and height) is 15 * 16 + 17 = 257.
                let mut points: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; GRID_WIDTH * GRID_WIDTH];

                for (chunk_index, chunk) in terrain.chunks.iter().enumerate() {
                    // Offsets in the world to be added to the coordinates
                    let chunk_x_offset =
                        block_x_offset + (chunk_index % BLOCK_WIDTH_IN_CHUNKS) as f32 * CHUNK_WIDTH;
                    let chunk_z_offset =
                        block_z_offset + (chunk_index / BLOCK_WIDTH_IN_CHUNKS) as f32 * CHUNK_WIDTH;

                    // Starting position in the `points` vector
                    let chunk_first_row = chunk_index / 16 * 16; // This makes sense thanks to the
                                                                 // integer division
                    let chunk_first_col = chunk_index % 16 * 16;

                    let interpolated_height_map =
                        terrain::interpolate_height_map(&chunk.height_map.to_vec());
                    let base_height = chunk.base_height;
                    interpolated_height_map
                        .into_iter()
                        .enumerate()
                        .for_each(|(index, height)| {
                            let position_x = chunk_x_offset
                                + (index % HEIGHT_MAP_WIDTH) as f32 * CHUNK_WIDTH / 16.0;
                            let position_z = chunk_z_offset
                                + (index / HEIGHT_MAP_WIDTH) as f32 * CHUNK_WIDTH / 16.0;

                            // Where to store this point in the `points` vector
                            let point_row_offset = index / HEIGHT_MAP_WIDTH;
                            let point_col_offset = index % HEIGHT_MAP_WIDTH;
                            let position_in_points_vec = (chunk_first_row + point_row_offset)
                                * GRID_WIDTH
                                + (chunk_first_col + point_col_offset);

                            points[position_in_points_vec] =
                                [position_x, height + base_height, position_z];
                        });
                }

                let rtin = Rtin::new(257);
                let tile = Tile::new(&points, &rtin);
                let (points, indices) = tile.get_mesh(
                    -1.0,
                    block_x_offset / RTIN_SCALE_FACTOR,
                    block_z_offset / RTIN_SCALE_FACTOR,
                    SCALE_FACTOR,
                );

                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
                mesh.set_indices(Some(Indices::U32(indices)));

                mesh.duplicate_vertices();
                mesh.compute_flat_normals();

                commands.spawn(PbrBundle {
                    mesh: meshes.add(mesh),
                    material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
                    ..default()
                });

                // Spawn WMOs converted to Bevy axis
                for wmo_mesh in asset.wmo_meshes.iter() {
                    let position = Vec3 {
                        x: -wmo_mesh.position.y / RTIN_SCALE_FACTOR,
                        y: wmo_mesh.position.z,
                        z: -wmo_mesh.position.x / RTIN_SCALE_FACTOR,
                    } * SCALE_FACTOR;

                    commands.spawn(PbrBundle {
                        mesh: meshes.add(shape::UVSphere::default().into()),
                        material: materials.add(Color::YELLOW.with_a(0.5).into()),
                        transform: Transform::from_translation(position)
                            .with_scale(Vec3::splat(0.01)),
                        ..default()
                    });

                    for group in wmo_mesh.groups.iter() {
                        let mut actual_mesh = Mesh::new(PrimitiveTopology::TriangleList);
                        actual_mesh.insert_attribute(
                            Mesh::ATTRIBUTE_POSITION,
                            group
                                .vertices
                                .iter()
                                .map(|v| {
                                    Vector3 {
                                        x: -v.y / RTIN_SCALE_FACTOR * SCALE_FACTOR + position.x,
                                        y: v.z / RTIN_SCALE_FACTOR * SCALE_FACTOR + position.y,
                                        z: -v.x / RTIN_SCALE_FACTOR * SCALE_FACTOR + position.z,
                                    }
                                    .as_array()
                                })
                                .collect::<Vec<[f32; 3]>>(),
                        );
                        actual_mesh.set_indices(Some(Indices::U16(
                            group.indices.clone().into_iter().flatten().collect(),
                        )));

                        actual_mesh.duplicate_vertices();
                        actual_mesh.compute_flat_normals();

                        commands.spawn(PbrBundle {
                            mesh: meshes.add(actual_mesh),
                            material: materials.add(Color::RED.with_a(1.0).into()),
                            ..default()
                        });
                    }
                }
            }
            _ => (),
        }
    }
}
