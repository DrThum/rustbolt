use std::sync::Arc;

use log::{error, trace};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use wow_srp::tbc_header::HeaderCrypto;

use crate::{
    protocol::client::{ClientMessage, ClientMessageHeader},
    WorldSocketError,
};

pub struct WorldSocket {
    pub write_half: Arc<Mutex<WriteHalf<TcpStream>>>,
    pub read_half: Arc<Mutex<ReadHalf<TcpStream>>>,
    pub encryption: Arc<Mutex<HeaderCrypto>>,
    pub account_id: u32,
}

impl WorldSocket {
    pub async fn read_packet(&self) -> Result<ClientMessage, WorldSocketError> {
        let mut buf = [0_u8; 6];
        let mut socket = self.read_half.lock().await;

        match socket.read(&mut buf[..6]).await {
            Ok(0) => {
                trace!("Client disconnected");
                return Err(WorldSocketError::ClientDisconnected);
            }
            Ok(n) if n < 6 => {
                error!("Received less than 6 bytes, need to handle partial header");
                return Err(WorldSocketError::SocketError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Received an incomplete client header",
                )));
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
                return Err(WorldSocketError::SocketError(e));
            }
        }
    }

    pub async fn shutdown(&self) {
        self.write_half
            .lock()
            .await
            .shutdown()
            .await
            .expect("Failed to shutdown WorldSocket");
    }
}
