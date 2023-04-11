use tokio::sync::RwLock;

use crate::session::world_session::WorldSession;

// TODO:
// - Keep an Arc<Map> in WorldSession for fast access
pub struct Map {
    id: u32,
    instance_id: Option<u32>,
    sessions: RwLock<Vec<WorldSession>>,
}

impl Map {
    pub fn new(id: u32, instance_id: Option<u32>) -> Self {
        Self {
            id,
            instance_id,
            sessions: RwLock::new(Vec::new()),
        }
    }
}
