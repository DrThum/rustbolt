use std::sync::Arc;

use binrw::BinWriterExt;
use log::{error, trace};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::{
        mpsc::{error::SendError, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};
use wow_srp::tbc_header::HeaderCrypto;

use crate::{
    protocol::{
        client::{ClientMessage, ClientMessageHeader},
        server::ServerMessageHeader,
    },
    WorldSocketError,
};

pub struct WorldSocket {
    pub read_half: Arc<Mutex<ReadHalf<TcpStream>>>,
    pub encryption: Arc<Mutex<HeaderCrypto>>,
    pub account_id: u32,
    socket_to_session_tx: UnboundedSender<ClientMessage>,
}

impl WorldSocket {
    pub fn new(
        write_half: Arc<Mutex<WriteHalf<TcpStream>>>,
        read_half: Arc<Mutex<ReadHalf<TcpStream>>>,
        encryption: Arc<Mutex<HeaderCrypto>>,
        account_id: u32,
        mut rx: UnboundedReceiver<(ServerMessageHeader, Vec<u8>)>,
        socket_to_session_tx: UnboundedSender<ClientMessage>,
    ) -> WorldSocket {
        let encryption_clone = encryption.clone();
        tokio::spawn(async move {
            while let Some((header, payload)) = rx.recv().await {
                let mut socket = write_half.lock().await;
                let mut encryption = encryption_clone.lock().await;

                // info!(
                //     "Sending {:?} ({:#X})",
                //     Opcode::n(header.opcode).unwrap(),
                //     header.opcode
                // );
                let mut encrypted_header: Vec<u8> = Vec::new();
                encryption
                    .write_encrypted_server_header(
                        &mut encrypted_header,
                        header.size,
                        header.opcode,
                    )
                    .unwrap();

                let mut writer = binrw::io::Cursor::new(Vec::new());
                writer.write_le(&encrypted_header).unwrap();
                let packet = writer.get_mut();
                trace!("Payload for opcode {:X}: {:X?}", header.opcode, payload);
                packet.extend(payload);
                socket.write(packet).await.unwrap();
            }
        });

        WorldSocket {
            read_half,
            encryption,
            account_id,
            socket_to_session_tx,
        }
    }

    pub fn queue_client_message(
        &self,
        client_message: ClientMessage,
    ) -> Result<(), SendError<ClientMessage>> {
        self.socket_to_session_tx.send(client_message)
    }

    pub async fn read_packet(&self) -> Result<ClientMessage, WorldSocketError> {
        let mut buf = [0_u8; 6];
        let mut socket = self.read_half.lock().await;

        match socket.read(&mut buf[..6]).await {
            Ok(0) => {
                trace!("Client disconnected");
                Err(WorldSocketError::ClientDisconnected)
            }
            Ok(n) if n < 6 => {
                error!("Received less than 6 bytes, need to handle partial header");
                Err(WorldSocketError::SocketError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Received an incomplete client header",
                )))
            }
            Ok(_) => {
                let mut encryption = self.encryption.lock().await;
                let client_header: ClientMessageHeader =
                    encryption.decrypt_client_header(buf).into();

                let bytes_to_read: usize = client_header.size as usize - 4; // Client opcode is u32
                let mut buf_payload = [0_u8; 2048];
                if bytes_to_read > 0 {
                    socket
                        .read(&mut buf_payload[..bytes_to_read])
                        .await
                        .unwrap();

                    Ok(ClientMessage {
                        header: client_header,
                        payload: buf_payload[..bytes_to_read].to_vec(),
                    })
                } else {
                    Ok(ClientMessage {
                        header: client_header,
                        payload: vec![],
                    })
                }
            }
            Err(e) => {
                error!("Socket error, closing");
                Err(WorldSocketError::SocketError(e))
            }
        }
    }

    pub fn shutdown(&self) {
        // self.write_half
        //     .lock()
        //     .await
        //     .shutdown()
        //     .await
        //     .expect("Failed to shutdown WorldSocket");
    }
}
