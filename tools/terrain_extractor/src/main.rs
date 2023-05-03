use std::path::PathBuf;

use clap::Parser;
use terrain_extractor::{extract_adts, get_all_map_names, list_adts_to_extract};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let mut args = Cli::parse();
    args.client_base_dir.push("Data");
    args.dbc_dir.push("Map.dbc");

    let map_dbc_path = args.dbc_dir.to_str().unwrap();
    let map_names = get_all_map_names(map_dbc_path)?;

    let client_data_dir = args.client_base_dir.to_str().unwrap();
    let output_dir = args.output_dir.to_str().unwrap().to_owned();

    let adts_to_extract = list_adts_to_extract(client_data_dir, output_dir, map_names).await?;
    extract_adts(client_data_dir, adts_to_extract).await
}

#[derive(Parser)]
#[command(name = "Rustbolt Terrain Extractor")]
#[command(about = "Extracts required terrain files from the WoW client", long_about = None)]
struct Cli {
    /// Path to the client base folder (the one containing Wow.exe)
    #[arg(short, long)]
    client_base_dir: PathBuf,
    /// Path to a folder containing Map.dbc (extracted with dbc_extractor)
    #[arg(short, long)]
    dbc_dir: PathBuf,
    /// Where to extract the files to
    #[arg(short, long)]
    output_dir: PathBuf,
}
