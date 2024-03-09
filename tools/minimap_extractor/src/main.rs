use minimap_extractor::get_minimaps;
use tools_shared::mpq_manager::MPQManager;

use std::path::PathBuf;

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let mut args = Cli::parse();
    args.client_base_dir.push("Data");

    let client_data_dir = args.client_base_dir.to_str().unwrap();
    let output_dir = args.output_dir.to_str().unwrap();
    let manager = MPQManager::new(client_data_dir)?;
    let minimaps = get_minimaps(&manager).await?;

    let extract_all_maps = args.specific_maps.is_empty();

    for (map_name, minimap) in minimaps.iter() {
        if extract_all_maps || args.specific_maps.contains(map_name) {
            minimap
                .extract_to_disk(
                    &manager,
                    output_dir,
                    !args.skip_extract_tiles,
                    !args.skip_stitch_maps,
                )
                .await;
        }
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
