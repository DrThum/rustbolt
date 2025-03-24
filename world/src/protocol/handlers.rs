use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};

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
    pub fn unhandled(_args: PacketHandlerArgs) {}
}
