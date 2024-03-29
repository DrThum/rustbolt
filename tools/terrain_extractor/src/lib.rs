extern crate nalgebra_glm as glm;

use std::{
    fs::{self, File},
    io::{Cursor, Read, Seek, SeekFrom, Write},
    mem::size_of,
    sync::{Arc, Mutex},
    thread::available_parallelism,
};

use binrw::BinWriterExt;
use bytemuck::cast_slice;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, warn};
use models::file_chunk::{adt::ADT, wdt::WDT, wmo::WMO};
use tokio::task::JoinHandle;
use tools_shared::mpq_manager::MPQManager;

mod models {
    pub mod file_chunk;
}

// Here, we implement a simplified DBC parsing to retrieve just the internal names of the maps. No
// need of the full implementation from the world crate.
pub fn get_all_map_names(map_dbc_path: &str) -> Result<Vec<String>, std::io::Error> {
    const SIZE_OF_HEADER: usize = 5 * size_of::<u32>();

    let mut file = File::open(map_dbc_path)?;

    let mut buffer = Vec::new();
    buffer.resize(SIZE_OF_HEADER, 0); // Magic + Header

    file.read(&mut buffer)?;

    assert!(
        buffer[..4] == [b'W', b'D', b'B', b'C'],
        "Provided Map.dbc is not a valid DBC file"
    );

    let buffer: Vec<u32> = cast_slice(&buffer).to_vec();
    let record_count = buffer[1];
    let record_size = buffer[3];
    let string_block_size = buffer[4];

    // Gather string offsets from the record (internal name is the second field of each record -> offset 4)
    let mut string_offsets: Vec<u32> = Vec::new();
    file.seek(SeekFrom::Start((SIZE_OF_HEADER + 4) as u64))?;
    for _i in 0..record_count {
        let mut buffer: [u8; 4] = [0_u8; 4];
        file.read(&mut buffer)?;

        string_offsets.push(u32::from_le_bytes(buffer));

        file.seek(SeekFrom::Current((record_size - 4) as i64))?; // Advance to the next record
    }

    // Read the whole string block
    let mut buffer = Vec::new();
    buffer.resize(string_block_size as usize, 0);
    file.seek(SeekFrom::End(-(string_block_size as i64)))?;
    file.read(&mut buffer)?;

    let mut strings: Vec<String> = Vec::new();
    strings.reserve_exact(record_count as usize);

    for offset in string_offsets.iter() {
        let offset = *offset as usize;
        let slice = &buffer[offset..];
        let str_end_index = slice.iter().position(|&c| c == 0).unwrap();

        let slice = &buffer[offset..(offset + str_end_index)];
        strings.push(std::str::from_utf8(&slice).unwrap().to_owned());
    }

    Ok(strings)
}

pub async fn list_adts_to_extract(
    client_data_dir: &str,
    output_dir: String,
    map_names: Vec<String>,
) -> Result<Vec<(String, String)>, std::io::Error> {
    let manager = MPQManager::new(client_data_dir)?;

    let mut adts_to_extract = Vec::new();
    for name in map_names {
        let wdt_path = format!("World\\Maps\\{}\\{}.wdt", name, name);
        if let Ok(Ok(Some(wdt_data))) = manager.get_file_data(wdt_path.clone()).await.await {
            if let Some(wdt) = read_wdt(&wdt_data) {
                if wdt.map_chunks.len() > 0 {
                    for coords in wdt.map_chunks {
                        let adt_file_name = format!(
                            "World\\Maps\\{}\\{}_{}_{}.adt",
                            name,
                            name,
                            coords.col, // FIXME: Why is it inverted
                            coords.row  // here? (MaNGOS does it)
                        );

                        let terrain_file_path = format!(
                            "{}/{}_{}_{}.terrain",
                            output_dir, name, coords.row, coords.col
                        );

                        adts_to_extract.push((adt_file_name, terrain_file_path));
                    }
                }
            }
        }
    }

    Ok(adts_to_extract)
}

pub async fn extract_adts(
    client_data_dir: &str,
    adts_to_extract: Vec<(String, String)>,
) -> Result<(), std::io::Error> {
    let prog_bar = ProgressBar::new(adts_to_extract.len() as u64);
    let progstyle =
        ProgressStyle::with_template("[{elapsed_precise}] {prefix} {wide_bar} {pos}/{len}")
            .unwrap();

    prog_bar.set_style(progstyle);
    prog_bar.set_prefix("Extracted map tiles");
    prog_bar.tick();
    let prog_bar = Arc::new(Mutex::new(prog_bar));

    // Spread data to extract across (2 * core count) threads
    let parallelism = available_parallelism().unwrap().get();
    let tiles_per_thread =
        ((adts_to_extract.len() as f32 / parallelism as f32).ceil() / 2.0) as usize;
    let tiles_per_thread = tiles_per_thread.max(1);

    let mut join_handles: Vec<JoinHandle<_>> = Vec::new();
    let groups: Vec<Vec<(String, String)>> = adts_to_extract
        .chunks(tiles_per_thread)
        .map(|c| c.to_owned())
        .collect();
    for group in groups {
        let prog_bar = prog_bar.clone();
        let manager = MPQManager::new(client_data_dir)?;

        let handle = tokio::spawn(async move {
            for (adt_file_name, terrain_file_path) in group {
                let adt_data = manager
                    .get_file_data(adt_file_name.to_string())
                    .await
                    .await
                    .unwrap()
                    .unwrap();

                if let Some(adt) = adt_data.and_then(|data| read_adt(&data)) {
                    let terrain_block = adt.terrain_block();
                    let mut file = fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(terrain_file_path)
                        .unwrap();
                    let mut writer = Cursor::new(Vec::new());
                    writer.write_le(&terrain_block).unwrap();

                    let wmos_to_extract = adt.wmos_to_extract();
                    writer.write_le(&(wmos_to_extract.len() as u32)).unwrap();
                    for wmo_to_extract in wmos_to_extract.iter() {
                        if let Some(wmo) = read_wmo(&manager, &wmo_to_extract.wmo_root_path).await {
                            let mesh = wmo.export_mesh(wmo_to_extract);
                            writer.write_le(&mesh).unwrap();
                        } else {
                            error!(
                                "failed to read wmo data at {}",
                                wmo_to_extract.wmo_root_path
                            );
                        }
                    }

                    file.write_all(writer.get_ref()).unwrap();
                } else {
                    warn!("failed to read ADT data");
                }

                prog_bar.lock().unwrap().inc(1);
            }
        });

        join_handles.push(handle);
    }

    tokio::join!(join_all(join_handles));

    prog_bar.lock().unwrap().finish();

    Ok(())
}

pub fn read_wdt(raw: &Vec<u8>) -> Option<WDT> {
    if !raw.is_empty() {
        WDT::parse(raw)
    } else {
        None
    }
}

pub fn read_adt(raw: &Vec<u8>) -> Option<ADT> {
    if !raw.is_empty() {
        ADT::parse(raw)
    } else {
        None
    }
}

pub async fn read_wmo(manager: &MPQManager, wmo_root_path: &String) -> Option<WMO> {
    let wmo_root_data = manager
        .get_file_data(wmo_root_path.clone())
        .await
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let root = WMO::parse_root(&wmo_root_data).unwrap();

    let mut groups = Vec::new();
    for group_index in 0..root.group_count {
        let group_path = wmo_root_path
            .clone()
            .replace(".wmo", &format!("_{group_index:03}.wmo"));
        let wmo_group_data = manager
            .get_file_data(group_path)
            .await
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let group = WMO::parse_group(wmo_group_data).unwrap();
        groups.push(group);
    }

    Some(WMO { groups })
}
