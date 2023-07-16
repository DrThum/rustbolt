use std::{sync::Arc, time::Duration};

use shipyard::Unique;
use tokio::time::Instant;

use crate::{
    chat_commands::ChatCommands, config::WorldConfig, database_context::DatabaseContext,
    session::opcode_handler::OpcodeHandler, DataStore, SessionHolder,
};

use super::{map_manager::MapManager, spell_effect_handler::SpellEffectHandler};

pub struct WorldContext {
    pub data_store: Arc<DataStore>,
    pub database: Arc<DatabaseContext>,
    pub opcode_handler: Arc<OpcodeHandler>,
    pub spell_effect_handler: Arc<SpellEffectHandler>,
    pub config: Arc<WorldConfig>,
    pub start_time: Instant,
    pub session_holder: Arc<SessionHolder>,
    pub map_manager: Arc<MapManager>,
    pub chat_commands: ChatCommands,
}

impl WorldContext {
    // Return the elapsed time since the World started
    pub fn game_time(&self) -> Duration {
        self.start_time.elapsed()
    }
}

#[derive(Unique)]
pub struct WrappedWorldContext(pub Arc<WorldContext>);
