use std::sync::Arc;

use crate::game::world_context::WorldContext;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;

mod account_handlers;
mod character_handlers;
mod item_handlers;
mod misc_handlers;
mod synchronization_handlers;

impl OpcodeHandler {
    pub async fn unhandled(
        _session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        _data: Vec<u8>,
    ) {
    }
}
