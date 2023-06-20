use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

#[binwrite]
#[server_opcode]
pub struct SmsgAuthChallenge {
    pub server_seed: u32,
}

#[binread]
#[derive(Debug)]
pub struct CmsgAuthSession {
    pub _build: u32,
    pub _server_id: u32,
    pub username: NullString,
    pub client_seed: u32,
    pub client_proof: [u8; 20],
}

impl CmsgAuthSession {
    pub fn len(&self) -> usize {
        4 + 4 + self.username.len() + 1 + 4 + 20
    }
}

#[binwrite]
#[server_opcode]
pub struct SmsgAuthResponse {
    pub result: u8,
    pub billing_time: u32,
    pub billing_flags: u8,
    pub billing_rested: u32,
    pub expansion: u8, // 0 = Vanilla, 1 = TBC
    pub position_in_queue: u32,
}

#[binread]
#[derive(Debug)]
pub struct ClientAddonInfo {
    pub name: NullString,
    pub crc: u32,
    pub unk1: u32,
    pub unk2: u8,
}

impl ClientAddonInfo {
    pub fn len(&self) -> usize {
        self.name.len() + 1 + 4 + 4 + 1
    }
}

#[binwrite]
#[derive(Copy, Clone)]
pub struct ServerAddonInfo {
    pub state: u8, // 2
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub use_crc_or_public_key: bool, // bool
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub use_public_key: bool, // bool, true if crc != standard crc
    pub public_key: Option<[u8; 256]>, // if use_public_key = 1
    pub unk: Option<u32>, // 0, if use_crc_of_public_key = 1
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub use_url: bool, // Always 0
}

#[binwrite]
#[server_opcode]
pub struct SmsgAddonInfo {
    pub addon_infos: Vec<ServerAddonInfo>,
}
