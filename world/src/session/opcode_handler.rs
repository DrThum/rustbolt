use futures::FutureExt;
use log::{error, trace};
use std::{collections::HashMap, sync::Arc};

use futures::future::BoxFuture;

use crate::{game::world_context::WorldContext, protocol::opcodes::Opcode};

use super::world_session::WorldSession;

pub type PacketHandler = Box<
    dyn Send + Sync + Fn(Arc<WorldSession>, Arc<WorldContext>, Vec<u8>) -> BoxFuture<'static, ()>,
>;

macro_rules! define_handler {
    ($opcode:expr, $handler:expr) => {
        (
            $opcode as u32,
            Box::new(|session, ctx, data| $handler(session, ctx, data).boxed()) as PacketHandler,
        )
    };
}

pub struct OpcodeHandler {
    handlers: HashMap<u32, PacketHandler>,
}

impl OpcodeHandler {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::from([
                define_handler!(Opcode::MsgNullAction, OpcodeHandler::unhandled),
                define_handler!(
                    Opcode::CmsgCharCreate,
                    OpcodeHandler::handle_cmsg_char_create
                ),
                define_handler!(Opcode::CmsgCharEnum, OpcodeHandler::handle_cmsg_char_enum),
                define_handler!(
                    Opcode::CmsgCharDelete,
                    OpcodeHandler::handle_cmsg_char_delete
                ),
                define_handler!(
                    Opcode::CmsgPlayerLogin,
                    OpcodeHandler::handle_cmsg_player_login
                ),
                define_handler!(Opcode::CmsgPing, OpcodeHandler::handle_cmsg_ping),
                define_handler!(
                    Opcode::CmsgRealmSplit,
                    OpcodeHandler::handle_cmsg_realm_split
                ),
                define_handler!(
                    Opcode::CmsgLogoutRequest,
                    OpcodeHandler::handle_cmsg_logout_request
                ),
                define_handler!(
                    Opcode::CmsgItemQuerySingle,
                    OpcodeHandler::handle_cmsg_item_query_single
                ),
                define_handler!(Opcode::CmsgNameQuery, OpcodeHandler::handle_cmsg_name_query),
                define_handler!(Opcode::CmsgQueryTime, OpcodeHandler::handle_cmsg_query_time),
                define_handler!(
                    Opcode::CmsgUpdateAccountData,
                    OpcodeHandler::handle_cmsg_update_account_data
                ),
                define_handler!(
                    Opcode::CmsgTimeSyncResp,
                    OpcodeHandler::handle_time_sync_resp
                ),
            ]),
        }
    }

    pub fn get_handler(&self, opcode: u32) -> &PacketHandler {
        self.handlers
            .get(&opcode)
            .map(|h| {
                trace!("Received {:?} ({:#X})", Opcode::n(opcode).unwrap(), opcode);
                h
            })
            .unwrap_or_else(|| {
                error!(
                    "Received unhandled {:?} ({:#X})",
                    Opcode::n(opcode).unwrap(),
                    opcode
                );
                self.handlers.get(&(Opcode::MsgNullAction as u32)).unwrap()
            })
    }
}
