use binrw::io::Cursor;
use binrw::{binwrite, BinWrite, BinWriterExt};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[binwrite]
#[derive(Debug)]
pub struct ServerMessageHeader {
    #[bw(big)]
    pub size: u16,
    pub opcode: u16,
}

pub struct ServerMessage<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE>> {
    payload: Payload,
}

pub trait ServerMessagePayload<const OPCODE: u16>: for<'a> BinWrite<Args<'a> = ()> {
    fn encode(&self) -> Result<Vec<u8>, binrw::Error> {
        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&self)?;
        Ok(writer.get_ref().to_vec())
    }
}

impl<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE> + BinWrite>
    ServerMessage<OPCODE, Payload>
{
    pub fn new(payload: Payload) -> Self {
        ServerMessage { payload }
    }

    pub async fn send_unencrypted(self, socket: &mut TcpStream) -> Result<(), binrw::Error> {
        let payload = self.payload.encode()?;
        let header = ServerMessageHeader {
            size: payload.len() as u16 + 2, // + 2 for the opcode size
            opcode: OPCODE,
        };

        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&header)?;
        let packet = writer.get_mut();
        packet.extend(payload);
        socket.write(packet).await?;
        Ok(())
    }

    pub fn encode_payload(&self) -> Result<Vec<u8>, binrw::Error> {
        self.payload.encode()
    }
}
