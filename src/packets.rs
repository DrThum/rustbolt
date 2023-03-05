use tokio::{io::AsyncWriteExt, net::TcpStream};
use wow_srp::{
    normalized_string::NormalizedString,
    server::{SrpProof, SrpVerifier},
};

#[derive(Debug)]
pub struct CmdAuthLogonChallengeClient {
    _opcode: u8,
    _protocol_version: u8,
    _size: u16,
    _game_name: String,
    _version: [u8; 3],
    _build: u16,
    _platform: String,
    _os: String,
    _locale: String,
    _worldregion_bias: u32,
    _ip: [u8; 4], // u32 on wowdev.wiki
    _account_name_length: u8,
    pub account_name: String,
}

impl CmdAuthLogonChallengeClient {
    pub fn new(buf: &Vec<u8>) -> CmdAuthLogonChallengeClient {
        let account_name_length: usize = buf[33].into();

        CmdAuthLogonChallengeClient {
            _opcode: buf[0],
            _protocol_version: buf[1],
            _size: u16::from_le_bytes([buf[2], buf[3]]),
            _game_name: std::str::from_utf8(&[buf[4], buf[5], buf[6], buf[7]])
                .unwrap()
                .to_string(),
            _version: [buf[8], buf[9], buf[10]],
            _build: u16::from_le_bytes([buf[11], buf[12]]),
            _platform: std::str::from_utf8(&[buf[13], buf[14], buf[15], buf[16]])
                .unwrap()
                .to_string(),
            _os: std::str::from_utf8(&[buf[17], buf[18], buf[19], buf[20]])
                .unwrap()
                .to_string(),
            _locale: std::str::from_utf8(&[buf[21], buf[22], buf[23], buf[24]])
                .unwrap()
                .to_string(),
            _worldregion_bias: u32::from_le_bytes([buf[25], buf[26], buf[27], buf[28]]),
            _ip: [buf[29], buf[30], buf[31], buf[32]],
            _account_name_length: buf[33],
            account_name: std::str::from_utf8(&buf[34..(34 + account_name_length)])
                .unwrap()
                .to_string(),
        }
    }
}

#[derive(Debug)]
pub struct CmdAuthLogonChallengeServer {
    _opcode: u8,
    _protocol_version: u8,
    _result: u8,
    _server_public_key: [u8; 32],
    _generator_len: u8, // Always 1
    _generator: u8,
    _large_safe_prime_len: u8,
    _large_safe_prime: Box<[u8]>,
    _salt: [u8; 32],
    _crc_salt: [u8; 16],
    _security_flags: u8,
}

impl CmdAuthLogonChallengeServer {
    fn get_proof(username: &str) -> SrpProof {
        let username = NormalizedString::new(username.to_string()).unwrap();
        let password = NormalizedString::new(username.to_string()).unwrap();
        SrpVerifier::from_username_and_password(username, password).into_proof()
    }

    pub fn new(username: &str) -> CmdAuthLogonChallengeServer {
        let p = Self::get_proof(username);

        CmdAuthLogonChallengeServer {
            _opcode: 0,
            _protocol_version: 0,
            _result: 0,
            _server_public_key: *p.server_public_key(),
            _generator_len: 1,
            _generator: wow_srp::GENERATOR,
            _large_safe_prime_len: wow_srp::LARGE_SAFE_PRIME_LENGTH,
            _large_safe_prime: Box::new(wow_srp::LARGE_SAFE_PRIME_LITTLE_ENDIAN),
            _salt: *p.salt(),
            _crc_salt: [0; 16],
            _security_flags: 0,
        }
    }

    pub async fn write(&self, socket: &mut TcpStream) -> Result<(), std::io::Error> {
        socket.write(&self._opcode.to_le_bytes()).await?;
        socket.write(&self._protocol_version.to_le_bytes()).await?;
        socket.write(&self._result.to_le_bytes()).await?;
        for i in self._server_public_key.iter() {
            socket.write_all(&i.to_le_bytes()).await?;
        }
        socket.write(&self._generator_len.to_le_bytes()).await?;
        socket.write(&self._generator.to_le_bytes()).await?;
        socket
            .write(&self._large_safe_prime_len.to_le_bytes())
            .await?;
        for i in self._large_safe_prime.iter() {
            socket.write_all(&i.to_le_bytes()).await?;
        }
        for i in self._salt.iter() {
            socket.write_all(&i.to_le_bytes()).await?;
        }
        for i in self._crc_salt.iter() {
            socket.write_all(&i.to_le_bytes()).await?;
        }
        socket.write(&self._security_flags.to_le_bytes()).await?;

        Ok(())
    }
}
