use std::path::PathBuf;

use clap::Parser;
use terrain_extractor::get_all_map_names;

fn main() -> Result<(), std::io::Error> {
    let mut args = Cli::parse();
    args.client_base_dir.push("Data");
    args.dbc_dir.push("Map.dbc");

    let client_data_dir = args.client_base_dir.to_str().unwrap();
    let map_dbc_path = args.dbc_dir.to_str().unwrap();
    let _output_dir = args.output_dir.to_str().unwrap();

    let map_names = get_all_map_names(map_dbc_path)?;

    let mut mpq_context = tools_shared::open_mpqs(client_data_dir)?;
    // let wdt_paths: Vec<String> = map_names
    let _: Vec<u8> = map_names[0..1] // REMOVEME
        .into_iter()
        .map(|name| {
            let wdt_path = format!("World\\Maps\\{}\\{}.wdt", name, name);
            if let Ok(Some(wdt_data)) =
                tools_shared::get_file_data(wdt_path.clone(), &mut mpq_context)
            {
                if let Some(wdt) = terrain_extractor::read_wdt(&wdt_data) {
                    for coords in wdt.map_chunks {
                        let adt_data = tools_shared::get_file_data(
                            format!(
                                "World\\Maps\\{}\\{}_{}_{}.adt",
                                name, name, coords.col, coords.row
                            ),
                            &mut mpq_context,
                        )
                        .unwrap();

                        println!(
                            "{} {} is_some: {}",
                            coords.row,
                            coords.col,
                            adt_data.is_some()
                        );

                        if let Some(adt_data) = adt_data {
                            println!("\tlen {}", adt_data.len());
                            terrain_extractor::read_adt(&adt_data).unwrap();
                        }
                    }
                }
            }

            0
        })
        .collect();

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
