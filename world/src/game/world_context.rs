use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    config::WorldConfig, database_context::DatabaseContext, session::opcode_handler::OpcodeHandler,
    DataStore,
};

pub struct WorldContext {
    pub data_store: Arc<DataStore>,
    pub database: Arc<DatabaseContext>,
    pub opcode_handler: Arc<OpcodeHandler>,
    pub config: Arc<WorldConfig>,
    pub start_time: Instant,
}

impl WorldContext {
    // Return the elapsed time since the World started
    pub fn game_time(&self) -> Duration {
        self.start_time.elapsed()
    }
}
