use image::{DynamicImage, GenericImage};
use image_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use minimap_extractor::{extract_tile_info_from_trs_line, get_trs_lines, Bounds};
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

    while let Ok(_) = cursor.read_until(0x0A as u8, &mut buffer) {
        if let Ok(line) = std::str::from_utf8(&buffer) {
            if line.is_empty() {
                break;
            }

            // TEMP REMOVEME
            if line.starts_with("WMO") {
                buffer.clear();
                bounds.reset();
                tiles.clear();

                continue;
            }

            // We start the processing of a new map
            if line.starts_with("dir:") {
                if !tiles.is_empty() {
                    println!("\tStitching map ({} tiles)...", tiles.len());
                    // TODO: for non-WMO maps each tile is 256x256, but for WMOs we're gonna need
                    // data from the MOGI header

                    let stitched_width_px = (bounds.end_x - bounds.start_x + 1) * 256;
                    let stitched_height_px = (bounds.end_y - bounds.start_y + 1) * 256;

                    let mut stitched =
                        DynamicImage::new_rgba16(stitched_width_px, stitched_height_px);

                    for (_, x, y, tile) in &tiles {
                        stitched
                            .copy_from(
                                tile,
                                (*x - bounds.start_x) * 256,
                                (*y - bounds.start_y) * 256,
                            )
                            .unwrap();
                    }

                    stitched
                        .save(format!("out/{}_full.png", &tiles.first().unwrap().0))
                        .unwrap();
                }

                print!("Processing Map {}", line.replace("dir: ", ""));
                buffer.clear();
                bounds.reset();
                tiles.clear();

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

                tiles.push((
                    tile_info.map_name.to_string(),
                    tile_info.tile_x,
                    tile_info.tile_y,
                    image,
                ));
            }
            // image
            //     .save(format!(
            //         "{}/{}",
            //         args.output_dir.to_str().unwrap(),
            //         tile_name.replace("\\", "_").replace(".blp", ".png")
            //     ))
            //     .unwrap();
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
}
