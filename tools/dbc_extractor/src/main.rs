use std::path::PathBuf;

use clap::Parser;

fn main() -> Result<(), std::io::Error> {
    let mut args = Cli::parse();
    args.client_base_dir.push("Data");

    let client_data_dir = args.client_base_dir.to_str().unwrap();
    let dbc_files = vec![
        "DBFilesClient\\ChrRaces.dbc",
        "DBFilesClient\\ChrClasses.dbc",
        "DBFilesClient\\CharStartOutfit.dbc",
        "DBFilesClient\\Item.dbc",
        "DBFilesClient\\Map.dbc",
        "DBFilesClient\\EmotesText.dbc",
        "DBFilesClient\\Spell.dbc",
        "DBFilesClient\\SkillLine.dbc",
        "DBFilesClient\\SkillLineAbility.dbc",
        "DBFilesClient\\Faction.dbc",
    ]
    .into_iter()
    .map(|f| f.to_owned())
    .collect();
    let output_dir = args.output_dir.to_str().unwrap();

    tools_shared::extract_files(client_data_dir, dbc_files, output_dir)
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
