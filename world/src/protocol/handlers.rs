use std::sync::Arc;

use shipyard::AllStoragesViewMut;

use crate::game::world_context::WorldContext;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;

mod account_handlers;
mod character_handlers;
mod chat_handlers;
mod combat_handlers;
mod gossip_handlers;
mod item_handlers;
mod loot_handlers;
mod misc_handlers;
mod movement_handlers;
mod query_handlers;
mod quest_handlers;
mod spell_handlers;
mod synchronization_handlers;

impl OpcodeHandler {
    pub fn unhandled(
        _session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        _data: Vec<u8>,
        _vm_all_storages: Option<AllStoragesViewMut>,
    ) {
    }
}
