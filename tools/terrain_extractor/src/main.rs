use binrw::{io::Cursor, BinWriterExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use std::{fs, io::Write, path::PathBuf};

use clap::Parser;
use log::warn;
use terrain_extractor::get_all_map_names;

fn main() -> Result<(), std::io::Error> {
    let mut args = Cli::parse();
    args.client_base_dir.push("Data");
    args.dbc_dir.push("Map.dbc");

    let client_data_dir = args.client_base_dir.to_str().unwrap();
    let map_dbc_path = args.dbc_dir.to_str().unwrap();
    let output_dir = args.output_dir.to_str().unwrap();
    let map_names = get_all_map_names(map_dbc_path)?;

    let progbar_parent = MultiProgress::new();
    let progstyle =
        ProgressStyle::with_template("[{elapsed_precise}] {prefix} {wide_bar} {pos}/{len}")
            .unwrap();

    let progbar_global = progbar_parent.add(ProgressBar::new(map_names.len() as u64));
    progbar_global.set_style(progstyle.clone());
    progbar_global.set_prefix("Extracted maps");
    progbar_global.tick(); // Required to make the  appear

    let mpq_context = tools_shared::open_mpqs(client_data_dir)?;
    // FIXME: Cannot make it parallel at the moment because the MPQContext (and thus the file
    // handles) is shared
    // TODO: Make num_threads a CLI parameter
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build_global()
        .unwrap();
    map_names.par_iter().for_each(|name| {
        let wdt_path = format!("World\\Maps\\{}\\{}.wdt", name, name);
        if let Ok(Some(wdt_data)) = tools_shared::get_file_data(wdt_path.clone(), &mpq_context) {
            if let Some(wdt) = terrain_extractor::read_wdt(&wdt_data) {
                if wdt.map_chunks.len() > 0 {
                    let progbar_this_map =
                        progbar_parent.add(ProgressBar::new(wdt.map_chunks.len() as u64));
                    progbar_this_map.set_style(progstyle.clone());
                    progbar_this_map.set_prefix(name.clone());

                    for coords in wdt.map_chunks {
                        let adt_file_name = format!(
                            "World\\Maps\\{}\\{}_{}_{}.adt",
                            name,
                            name,
                            coords.col, // FIXME: Why is it inverted
                            coords.row  // here? (MaNGOS does it)
                        );
                        let adt_data = tools_shared::get_file_data(adt_file_name, &mpq_context).unwrap();

                        if let Some(terrain_block) =
                            adt_data.and_then(|data| terrain_extractor::read_adt(&data))
                        {
                            let mut file = fs::OpenOptions::new()
                                .create(true)
                                .write(true)
                                .open(format!(
                                    "{}/{}_{}_{}.terrain",
                                    output_dir, name, coords.row, coords.col
                                ))
                                .unwrap();
                            let mut writer = Cursor::new(Vec::new());
                            writer.write_le(&terrain_block).unwrap();

                            file.write_all(writer.get_ref()).unwrap();
                        } else {
                            warn!("failed to extract terrain info");
                        }

                        progbar_this_map.inc(1);
                        progbar_global.tick(); // Refresh the elapsed time
                    }

                    progbar_this_map.finish();
                }
            }
        }

        progbar_global.inc(1);
    });

    progbar_global.finish();

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
