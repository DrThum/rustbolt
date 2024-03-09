use image::{DynamicImage, GenericImage};
use regex::Regex;
use tools_shared::mpq_manager::MPQManager;

pub mod models {
    pub mod bounds;
    pub mod tile_info;
}

use models::{bounds::Bounds, tile_info::TileInfo};

pub async fn get_trs_lines(manager: &MPQManager) -> Vec<u8> {
    manager
        .get_file_data("textures\\Minimap\\md5translate.trs".to_string())
        .await
        .await
        .expect("unable to find md5translate.trs file")
        .expect("unable to find md5translate.trs file")
        .expect("unable to find md5translate.trs file") // lol
}

pub fn extract_tile_info_from_trs_line(line: &str) -> Option<TileInfo> {
    let parts: Vec<&str> = line.split("\t").collect();
    let hashed_file_name = parts[1]; // The actual file in the MPQ with the MD5 hash as a name
    let tile_name = parts[0]; // The map tile it represents

    let re = Regex::new("(.*)\\\\map([0-9]+)_([0-9]+)\\.blp").unwrap();
    for (_, [map_name, tile_x, tile_y]) in re.captures_iter(tile_name).map(|c| c.extract()) {
        return Some(TileInfo {
            name: tile_name,
            hashed_file_name,
            map_name,
            tile_x: tile_x.parse().unwrap(),
            tile_y: tile_y.parse().unwrap(),
        });
    }

    // panic!("unable to extract TileInfo from md5translate.trs line");
    None
}

pub fn stitch_map_tiles(
    tiles: &Vec<(String, u32, u32, DynamicImage)>,
    bounds: &Bounds,
    output_dir: &str,
) {
    if tiles.is_empty() {
        return;
    }

    println!("\tStitching map ({} tiles)...", tiles.len());
    // TODO: for non-WMO maps each tile is 256x256, but for WMOs we're gonna need
    // data from the MOGI header

    let stitched_width_px = (bounds.end_x - bounds.start_x + 1) * 256;
    let stitched_height_px = (bounds.end_y - bounds.start_y + 1) * 256;

    let mut stitched = DynamicImage::new_rgba16(stitched_width_px, stitched_height_px);

    for (_, x, y, tile) in tiles {
        stitched
            .copy_from(
                tile,
                (*x - bounds.start_x) * 256,
                (*y - bounds.start_y) * 256,
            )
            .unwrap();
    }

    stitched
        .save(format!(
            "{}/{}/{}_full.png",
            output_dir,
            &tiles.first().unwrap().0,
            &tiles.first().unwrap().0
        ))
        .unwrap();
}

pub fn extract_tile(tile_info: &TileInfo, image: &DynamicImage, output_dir: &str) {
    // TODO: Only do this once
    std::fs::create_dir_all(format!("{}/{}", output_dir, tile_info.map_name))
        .expect("failed to create output dir");

    image
        .save(format!(
            "{}/{}/{}",
            output_dir,
            tile_info.map_name,
            tile_info.name.replace("\\", "_").replace(".blp", ".png")
        ))
        .unwrap();
}
