use std::{collections::HashMap, io::BufRead};

use regex::Regex;
use tools_shared::mpq_manager::MPQManager;

pub mod models {
    pub mod bounds;
    pub mod minimap;
    pub mod tile_info;
}

use models::{minimap::Minimap, tile_info::TileInfo};

async fn get_trs_lines(manager: &MPQManager) -> Vec<u8> {
    manager
        .get_file_data("textures\\Minimap\\md5translate.trs".to_string())
        .await
        .await
        .expect("unable to find md5translate.trs file")
        .expect("unable to find md5translate.trs file")
        .expect("unable to find md5translate.trs file") // lol
}

fn extract_tile_info_from_trs_line(line: &str) -> Option<TileInfo> {
    let parts: Vec<&str> = line.split("\t").collect();
    let hashed_file_name = parts[1]; // The actual file in the MPQ with the MD5 hash as a name
    let tile_name = parts[0]; // The map tile it represents

    let re = Regex::new("(.*)\\\\map([0-9]+)_([0-9]+)\\.blp").unwrap();
    for (_, [map_name, tile_x, tile_y]) in re.captures_iter(tile_name).map(|c| c.extract()) {
        return Some(TileInfo {
            name: tile_name.to_string(),
            hashed_file_name: hashed_file_name.strip_suffix("\r\n").unwrap().to_string(),
            map_name: map_name.to_string(),
            tile_x: tile_x.parse().unwrap(),
            tile_y: tile_y.parse().unwrap(),
        });
    }

    // panic!("unable to extract TileInfo from md5translate.trs line");
    None
}

pub async fn get_minimaps(
    manager: &MPQManager,
) -> Result<HashMap<String, Minimap>, std::io::Error> {
    let trs = get_trs_lines(&manager).await;

    let mut cursor = std::io::Cursor::new(trs);
    let mut buffer = vec![];
    let mut current_map_name: String = "".to_string();

    let mut minimaps: HashMap<String, Minimap> = HashMap::new();

    while let Ok(_) = cursor.read_until(0x0A as u8, &mut buffer) {
        if let Ok(line) = std::str::from_utf8(&buffer) {
            if line.is_empty() {
                break;
            }

            // We start the processing of a new map
            if line.starts_with("dir:") {
                current_map_name = line
                    .replace("dir: ", "")
                    .strip_suffix("\r\n")
                    .unwrap()
                    .to_string();

                minimaps.insert(current_map_name.clone(), Minimap::new());
            } else {
                if let Some(tile_info) = extract_tile_info_from_trs_line(line) {
                    let minimap = minimaps
                        .get_mut(&current_map_name)
                        .expect("unknown minimap");

                    minimap.bounds.refresh(tile_info.tile_x, tile_info.tile_y);
                    minimap.tiles.push(tile_info);
                }
            }
        }

        buffer.clear();
    }

    Ok(minimaps)
}
