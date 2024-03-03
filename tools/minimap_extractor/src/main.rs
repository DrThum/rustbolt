use image_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use tools_shared::mpq_manager::MPQManager;

use std::{io::BufRead, path::PathBuf};

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let mut args = Cli::parse();
    args.client_base_dir.push("Data");

    let client_data_dir = args.client_base_dir.to_str().unwrap();
    let manager = MPQManager::new(client_data_dir)?;

    let trs = manager
        .get_file_data("textures\\Minimap\\md5translate.trs".to_string())
        .await
        .await
        .expect("unable to find md5translate.trs file")
        .expect("unable to find md5translate.trs file")
        .expect("unable to find md5translate.trs file"); // lol

    let mut cursor = std::io::Cursor::new(trs);
    let mut buffer = vec![];

    while let Ok(_) = cursor.read_until(0x0A as u8, &mut buffer) {
        if let Ok(line) = std::str::from_utf8(&buffer) {
            if line.is_empty() {
                break;
            }

            // We don't need these lines, the full path is included for each tile
            if line.starts_with("dir:") {
                buffer.clear();
                continue;
            }

            let parts: Vec<&str> = line.split("\t").collect();
            let tile_file = parts[1]; // The actual file in the MPQ with the MD5 hash as a name
            let tile_name = parts[0]; // The map tile it represents

            let mut full_path = format!("textures\\Minimap\\{}", tile_file);
            // Remove the trailing \r\n
            full_path.truncate(full_path.len() - 2);

            let blp_data = manager
                .get_file_data(full_path)
                .await
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            let blp_image = load_blp_from_buf(&blp_data).unwrap();
            let image = blp_to_image(&blp_image, 0).expect("BlpImage to DynamicImage failed");
            image
                .save(format!(
                    "{}/{}",
                    args.output_dir.to_str().unwrap(),
                    tile_name.replace("\\", "_").replace(".blp", ".png")
                ))
                .unwrap();
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
