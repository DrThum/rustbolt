use binrw::io::Cursor;
use binrw::{binwrite, BinWrite, BinWriterExt};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use wow_srp::tbc_header::HeaderCrypto;

#[binwrite]
struct ServerMessageHeader {
    #[bw(big)]
    pub size: u16,
    pub opcode: u16,
}

pub struct ServerMessage<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE> + BinWrite> {
    payload: Payload,
}

pub trait ServerMessagePayload<const OPCODE: u16> {
    fn encode(&self) -> Result<Vec<u8>, binrw::Error>
    where
        Self: BinWrite,
        for<'a> <Self as BinWrite>::Args<'a>: Default,
    {
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

    pub async fn send_unencrypted(self, socket: &mut TcpStream) -> Result<(), binrw::Error>
    where
        for<'a> <Payload as BinWrite>::Args<'a>: Default,
    {
        let payload = self.payload.encode()?;
        let header = ServerMessageHeader {
            size: payload.len() as u16 + 2, // + 2 for the opcode size
            opcode: OPCODE,
        };

        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&header)?;
        let packet = writer.get_mut();
        packet.extend(payload);
        socket.write(&packet).await?;
        Ok(())
    }

    // TODO: Refactor send and send_unencrypted to use a common fn _send(self, socket,
    // Option<HeaderCrypto>)
    pub async fn send(
        self,
        socket: &mut TcpStream,
        encryption: &mut HeaderCrypto,
    ) -> Result<(), binrw::Error>
    where
        for<'a> <Payload as BinWrite>::Args<'a>: Default,
    {
        let payload = self.payload.encode()?;
        let header = ServerMessageHeader {
            size: payload.len() as u16 + 2, // + 2 for the opcode size
            opcode: OPCODE,
        };
        let mut encrypted_header: Vec<u8> = Vec::new();
        encryption.write_encrypted_server_header(
            &mut encrypted_header,
            header.size,
            header.opcode,
        )?;

        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&encrypted_header)?;
        let packet = writer.get_mut();
        packet.extend(payload);
        socket.write(&packet).await?;
        Ok(())
    }
}
