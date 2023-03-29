use std::path::PathBuf;

use clap::Parser;
use dbc_extractor::extract;

fn main() -> Result<(), std::io::Error> {
    let mut args = Cli::parse();
    args.client_base_dir.push("Data/frFR/patch-frFR-2.MPQ");

    extract(
        args.client_base_dir.to_str().unwrap(),
        "DBFilesClient\\ChrRaces.dbc",
        args.output_dir.to_str().unwrap(),
    )
}

#[derive(Parser)]
#[command(name = "Rustbolt DBC Extractor")]
#[command(about = "Extracts required DBC files from the WoW client", long_about = None)]
struct Cli {
    /// Path to the client base folder (the one containing Wow.exe)
    #[arg(short, long)]
    client_base_dir: PathBuf,
    /// Where to extract the files to
    #[arg(short, long)]
    output_dir: PathBuf,
}
