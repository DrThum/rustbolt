use std::path::PathBuf;

use clap::Parser;
use terrain_extractor::get_all_map_names;

fn main() -> Result<(), std::io::Error> {
    let mut args = Cli::parse();
    args.client_base_dir.push("Data");
    args.dbc_dir.push("Map.dbc");

    let client_data_dir = args.client_base_dir.to_str().unwrap();
    let map_dbc_path = args.dbc_dir.to_str().unwrap();
    let output_dir = args.output_dir.to_str().unwrap();

    let map_names = get_all_map_names(map_dbc_path)?;
    let wdt_paths: Vec<String> = map_names
        .into_iter()
        .map(|name| format!("World\\Maps\\{}\\{}.wdt", name, name))
        .collect();

    shared::extract_files(client_data_dir, wdt_paths, output_dir)?;

    Ok(())
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
