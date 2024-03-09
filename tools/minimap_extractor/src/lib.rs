use regex::Regex;
use tools_shared::mpq_manager::MPQManager;

pub async fn get_trs_lines(manager: &MPQManager) -> Vec<u8> {
    manager
        .get_file_data("textures\\Minimap\\md5translate.trs".to_string())
        .await
        .await
        .expect("unable to find md5translate.trs file")
        .expect("unable to find md5translate.trs file")
        .expect("unable to find md5translate.trs file") // lol
}

pub fn extract_tile_info_from_trs_line(line: &str) -> Option<TileInfo> {
    let parts: Vec<&str> = line.split("\t").collect();
    let hashed_file_name = parts[1]; // The actual file in the MPQ with the MD5 hash as a name
    let tile_name = parts[0]; // The map tile it represents

    let re = Regex::new("(.*)\\\\map([0-9]+)_([0-9]+)\\.blp").unwrap();
    for (_, [map_name, tile_x, tile_y]) in re.captures_iter(tile_name).map(|c| c.extract()) {
        return Some(TileInfo {
            name: tile_name,
            hashed_file_name,
            map_name,
            tile_x: tile_x.parse().unwrap(),
            tile_y: tile_y.parse().unwrap(),
        });
    }

    // panic!("unable to extract TileInfo from md5translate.trs line");
    None
}

pub struct TileInfo<'a> {
    pub name: &'a str,
    pub hashed_file_name: &'a str,
    pub map_name: &'a str,
    pub tile_x: u32,
    pub tile_y: u32,
}

// Represents the min and max X and Y found for a given map (in mapXX_YY.blp)
pub struct Bounds {
    pub start_x: u32, // First tile
    pub start_y: u32,
    pub end_x: u32, // Last tile
    pub end_y: u32,
}

impl Bounds {
    pub fn reset(&mut self) {
        self.start_x = u32::MAX;
        self.start_y = u32::MAX;

        self.end_x = u32::MIN;
        self.end_y = u32::MIN;
    }

    // Enlarge bounds if needed
    pub fn refresh(&mut self, candidate_x: u32, candidate_y: u32) {
        self.start_x = candidate_x.min(self.start_x);
        self.start_y = candidate_y.min(self.start_y);

        self.end_x = candidate_x.max(self.end_x);
        self.end_y = candidate_y.max(self.end_y);
    }
}
