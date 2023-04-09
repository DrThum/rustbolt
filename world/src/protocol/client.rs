use binrw::io::Cursor;
use binrw::{binread, BinRead, BinReaderExt};
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

impl ClientMessage {
    pub fn read_as<T>(data: Vec<u8>) -> Result<T, binrw::Error>
    where
        T: for<'a> BinRead<Args<'a> = ()>,
    {
        let mut reader = Cursor::new(data);
        reader.read_le()
    }
}
