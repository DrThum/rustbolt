use binrw::io::Cursor;
use binrw::{binwrite, BinWrite, BinWriterExt};
use log::trace;
use miniz_oxide::deflate::CompressionLevel;
use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::MutexGuard;
use wow_srp::tbc_header::HeaderCrypto;

use crate::protocol::opcodes::Opcode;

#[binwrite]
pub(crate) struct ServerMessageHeader {
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
        socket.write(&packet).await?;
        Ok(())
    }

    pub async fn send(
        &self,
        socket: &mut MutexGuard<'_, WriteHalf<TcpStream>>,
        encryption: &mut MutexGuard<'_, HeaderCrypto>,
    ) -> Result<(), binrw::Error> {
        let payload = self.payload.encode()?;

        // TODO: Write a specialized impl for SmsgUpdateObject
        // Seems like not Rust does not support this at the moment:
        // https://github.com/rust-lang/rust/issues/31844
        if OPCODE == Opcode::SmsgUpdateObject as u16 && payload.len() > 50 {
            // Change to SMSG_COMPRESSED_UPDATE_OBJECT and compress the payload
            let uncompressed_size = payload.len();
            let compressed_payload: Vec<u8> = miniz_oxide::deflate::compress_to_vec_zlib(
                &payload,
                CompressionLevel::DefaultLevel as u8,
            );

            let header = ServerMessageHeader {
                size: compressed_payload.len() as u16 + 2 + 4, /* + 2 for opcode + 4 for uncompressed_size */
                opcode: Opcode::SmsgCompressedUpdateObject as u16,
            };

            let mut encrypted_header: Vec<u8> = Vec::new();
            encryption.write_encrypted_server_header(
                &mut encrypted_header,
                header.size,
                header.opcode,
            )?;

            let mut writer = Cursor::new(Vec::new());
            writer.write_le(&encrypted_header)?;
            writer.write_le(&(uncompressed_size as u32))?;
            let compressed_packet = writer.get_mut();
            trace!(
                "Payload for opcode SmsgCompressedUpdateObject (uncompressed size = {}): {:X?}",
                uncompressed_size,
                compressed_payload
            );
            compressed_packet.extend(compressed_payload);
            socket.write(&compressed_packet).await?;

            Ok(())
        } else {
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
            trace!("Payload for opcode {:X}: {:X?}", header.opcode, payload);
            packet.extend(payload);
            socket.write(&packet).await?;
            Ok(())
        }
    }
}
