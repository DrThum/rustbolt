use binrw::binread;
use wow_srp::tbc_header::ClientHeader;

#[binread]
#[derive(Debug)]
pub struct ClientMessageHeader {
    #[br(big)]
    pub size: u16,
    pub opcode: u32,
}

impl From<ClientHeader> for ClientMessageHeader {
    fn from(value: ClientHeader) -> Self {
        Self {
            size: value.size,
            opcode: value.opcode,
        }
    }
}

pub struct ClientMessage {
    pub header: ClientMessageHeader,
    pub payload: Vec<u8>,
}
