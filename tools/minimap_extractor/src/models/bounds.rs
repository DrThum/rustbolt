// Represents the min and max X and Y found for a given map (in mapXX_YY.blp)
pub struct Bounds {
    pub start_x: u32, // First tile
    pub start_y: u32,
    pub end_x: u32, // Last tile
    pub end_y: u32,
}

impl Bounds {
    pub fn new() -> Self {
        Self {
            start_x: u32::MAX,
            start_y: u32::MAX,
            end_x: u32::MIN,
            end_y: u32::MIN,
        }
    }

    // Enlarge bounds if needed
    pub fn refresh(&mut self, candidate_x: u32, candidate_y: u32) {
        self.start_x = candidate_x.min(self.start_x);
        self.start_y = candidate_y.min(self.start_y);

        self.end_x = candidate_x.max(self.end_x);
        self.end_y = candidate_y.max(self.end_y);
    }
}
