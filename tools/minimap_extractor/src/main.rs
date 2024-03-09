use image::DynamicImage;
use image_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use minimap_extractor::{
    extract_tile, extract_tile_info_from_trs_line, get_trs_lines, models::bounds::Bounds,
    stitch_map_tiles,
};
use tools_shared::mpq_manager::MPQManager;

use std::{io::BufRead, path::PathBuf};

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let mut args = Cli::parse();
    args.client_base_dir.push("Data");

    let client_data_dir = args.client_base_dir.to_str().unwrap();
    let manager = MPQManager::new(client_data_dir)?;

    let trs = get_trs_lines(&manager).await;
    let mut cursor = std::io::Cursor::new(trs);
    let mut buffer = vec![];

    let mut bounds = Bounds {
        start_x: 0,
        start_y: 0,
        end_x: 0,
        end_y: 0,
    };
    bounds.reset();

    let mut tiles: Vec<(String, u32, u32, DynamicImage)> = vec![];
    let mut current_map_name: String = "".to_string();

    while let Ok(_) = cursor.read_until(0x0A as u8, &mut buffer) {
        if let Ok(line) = std::str::from_utf8(&buffer) {
            if line.is_empty() {
                break;
            }

            // We start the processing of a new map
            if line.starts_with("dir:") {
                if !args.skip_stitch_maps {
                    stitch_map_tiles(&tiles, &bounds, args.output_dir.to_str().unwrap());
                }

                current_map_name = line.replace("dir: ", "");
                current_map_name.truncate(current_map_name.len() - 2);
                if !args.specific_maps.is_empty() && args.specific_maps.contains(&current_map_name)
                {
                    println!("Processing Map {current_map_name}");
                }
                buffer.clear();
                bounds.reset();
                tiles.clear();

                continue;
            }

            if !args.specific_maps.is_empty() && !args.specific_maps.contains(&current_map_name) {
                buffer.clear();
                continue;
            }

            if let Some(tile_info) = extract_tile_info_from_trs_line(line) {
                let mut full_path = format!("textures\\Minimap\\{}", tile_info.hashed_file_name);
                // Remove the trailing \r\n
                full_path.truncate(full_path.len() - 2);

                bounds.refresh(tile_info.tile_x, tile_info.tile_y);

                let blp_data = manager
                    .get_file_data(full_path)
                    .await
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                let blp_image = load_blp_from_buf(&blp_data).unwrap();
                let image = blp_to_image(&blp_image, 0).expect("BlpImage to DynamicImage failed");

                if !args.skip_extract_tiles {
                    extract_tile(&tile_info, &image, args.output_dir.to_str().unwrap());
                }

                tiles.push((
                    tile_info.map_name.to_string(),
                    tile_info.tile_x,
                    tile_info.tile_y,
                    image,
                ));
            }
        }

        buffer.clear();
    }

    Ok(())
}

#[derive(Parser)]
#[command(name = "Rustbolt Minimap Extractor")]
#[command(about = "Extracts minimap tiles from the WoW client as PNG", long_about = None)]
struct Cli {
    /// Path to the client base folder (the one containing Wow.exe)
    #[arg(short, long)]
    client_base_dir: PathBuf,
    /// Where to extract the files to
    #[arg(short, long)]
    output_dir: PathBuf,
    /// Whether to skip extracting independent tiles
    #[arg(short = 't', long)]
    skip_extract_tiles: bool,
    /// Whether to skip extracting stitched map images
    #[arg(short = 's', long)]
    skip_stitch_maps: bool,
    /// Specific maps to extract
    #[arg(short = 'm', long)]
    specific_maps: Vec<String>,
}
