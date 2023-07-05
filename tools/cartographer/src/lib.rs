use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use models::terrain::{self, WrappedTerrainBlock};
use resources::terrain_handle::TerrainHandle;
use shared::models::terrain_info::{BLOCK_WIDTH, CHUNK_WIDTH, MAP_WIDTH_IN_BLOCKS};
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
    for row in 30..34 {
        for col in 30..34 {
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
                mouse_translate_sensitivity: Vec2::splat(0.4),
                ..default()
            },
            Vec3::new(0.0, 5.0, 5.0),
            Vec3::new(0., 0., 0.),
            Vec3::Y,
        ));
}

pub const SCALE_FACTOR: f32 = 0.01;

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
                let terrain = &terrains.get(handle).unwrap().0;
                let (block_row, block_col) = terrain_handles.0.get(handle).unwrap();
                let block_x_offset =
                    BLOCK_WIDTH * (block_col - (MAP_WIDTH_IN_BLOCKS as i32 / 2)) as f32;
                let block_z_offset =
                    BLOCK_WIDTH * (block_row - (MAP_WIDTH_IN_BLOCKS as i32 / 2)) as f32;

                let mut points: Vec<[f32; 3]> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();

                for (chunk_index, chunk) in terrain.chunks.iter().enumerate() {
                    let hm = terrain::interpolate_height_map(&chunk.height_map.to_vec());
                    let base_height = chunk.base_height;
                    let chunk_z_offset = block_z_offset + (chunk_index / 16) as f32 * CHUNK_WIDTH;
                    let chunk_x_offset = block_x_offset + (chunk_index % 16) as f32 * CHUNK_WIDTH;

                    for row in 0..16 {
                        let z_offset = chunk_z_offset + row as f32 * CHUNK_WIDTH / 16.0;

                        for col in 0..16 {
                            let x_offset = chunk_x_offset + col as f32 * CHUNK_WIDTH / 16.0;
                            let base_hm_index = row * 17 + col;

                            let top_left = [x_offset, base_height + hm[base_hm_index], z_offset]
                                .map(|v| v * SCALE_FACTOR);
                            let top_right = [
                                x_offset + CHUNK_WIDTH / 16.0,
                                base_height + hm[base_hm_index + 1],
                                z_offset,
                            ]
                            .map(|v| v * SCALE_FACTOR);
                            let bottom_left = [
                                x_offset,
                                base_height + hm[base_hm_index + 17],
                                (z_offset + CHUNK_WIDTH / 16.0),
                            ]
                            .map(|v| v * SCALE_FACTOR);
                            let bottom_right = [
                                x_offset + CHUNK_WIDTH / 16.0,
                                base_height + hm[base_hm_index + 18],
                                (z_offset + CHUNK_WIDTH / 16.0),
                            ]
                            .map(|v| v * SCALE_FACTOR);

                            let indices_index_base: u32 = points.len() as u32;

                            points.push(top_left);
                            points.push(top_right);
                            points.push(bottom_left);
                            points.push(bottom_right);

                            // Indices:
                            //  We push the 5 relevant points to the `points` Vec then we build the
                            //  triangles from these 5 points, via the `set_incices` method.
                            //
                            // - Triangle 1 is top left, top right, bottom_right so 0 1 3
                            // - Triangle 2 is top left, bottom right, bottom left so 0 3 2
                            //
                            // Note: we have to draw them inverted to have the visible face up.
                            indices.extend_from_slice(
                                &[3, 1, 0, 2, 3, 0].map(|v| v + indices_index_base),
                            );
                        }
                    }
                }

                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                let points_number = points.len();
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; points_number]);
                mesh.set_indices(Some(Indices::U32(indices)));

                mesh.duplicate_vertices();
                mesh.compute_flat_normals();

                commands.spawn(PbrBundle {
                    mesh: meshes.add(mesh),
                    material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
                    ..default()
                });
            }
            _ => (),
        }
    }
}
