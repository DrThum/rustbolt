use std::{sync::Arc, time::Duration};

use atomic_counter::{AtomicCounter, RelaxedCounter};
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
    pub session_holder: Arc<SessionHolder<u32>>,
    pub map_manager: Arc<MapManager>,
    pub chat_commands: ChatCommands,
    pub next_item_guid_counter: RelaxedCounter,
}

impl WorldContext {
    // Return the elapsed time since the World started
    pub fn game_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn next_item_guid(&self) -> u32 {
        self.next_item_guid_counter.inc().try_into().unwrap()
    }
}

#[derive(Unique)]
pub struct WrappedWorldContext(pub Arc<WorldContext>);
