use image::{DynamicImage, GenericImage};
use image_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use tools_shared::mpq_manager::MPQManager;

use super::{bounds::Bounds, tile_info::TileInfo};

pub struct Minimap {
    pub tiles: Vec<TileInfo>,
    pub bounds: Bounds,
}

impl Minimap {
    pub fn new() -> Self {
        Self {
            tiles: vec![],
            bounds: Bounds::new(),
        }
    }

    pub async fn extract_to_disk(
        &self,
        manager: &MPQManager,
        output_dir: &str,
        extract_tiles: bool,
        extract_stitched: bool,
    ) {
        if self.tiles.is_empty() {
            return;
        }

        let map_name = &self.tiles.first().unwrap().map_name;
        println!("Extracting map {map_name}...");

        if !extract_tiles && !extract_stitched {
            println!("\tNothing to do");
            return;
        }

        std::fs::create_dir_all(format!("{}/{}", output_dir, map_name))
            .expect("failed to create output dir");

        let stitched_width_px = (self.bounds.end_x - self.bounds.start_x + 1) * 256;
        let stitched_height_px = (self.bounds.end_y - self.bounds.start_y + 1) * 256;

        let mut stitched = DynamicImage::new_rgba16(stitched_width_px, stitched_height_px);

        for tile in self.tiles.iter() {
            let full_path = format!("textures\\Minimap\\{}", tile.hashed_file_name);

            let blp_data = manager
                .get_file_data(full_path)
                .await
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            let blp_image = load_blp_from_buf(&blp_data).unwrap();
            let image = blp_to_image(&blp_image, 0).expect("BlpImage to DynamicImage failed");

            if extract_tiles {
                image
                    .save(format!(
                        "{}/{}/{}_{}_{}.png",
                        output_dir,
                        tile.map_name,
                        // tile.name.replace("\\", "_").replace(".blp", ".png")
                        tile.map_name,
                        32 - tile.tile_x as i32,
                        32 - tile.tile_y as i32,
                    ))
                    .unwrap();
            }

            if extract_stitched {
                stitched
                    .copy_from(
                        &image,
                        (tile.tile_x - self.bounds.start_x) * 256,
                        (tile.tile_y - self.bounds.start_y) * 256,
                    )
                    .unwrap();
            }
        }

        if extract_stitched {
            stitched
                .save(format!("{}/{}/{}_full.png", output_dir, map_name, map_name,))
                .unwrap();
        }

        println!("\tDone");
    }
}
