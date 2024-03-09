pub struct TileInfo<'a> {
    pub name: &'a str,
    pub hashed_file_name: &'a str,
    pub map_name: &'a str,
    pub tile_x: u32,
    pub tile_y: u32,
}
