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

    pub fn new(username: &str) -> (CmdAuthLogonChallengeServer, SrpProof) {
        let p = Self::get_proof(username);

        (
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
            },
            p,
        )
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

#[derive(Debug)]
pub struct CmdAuthLogonProofClient {
    _opcode: u8,
    _client_public_key: [u8; 32],
    _client_proof: [u8; 20],
    _crc_hash: [u8; 20],
    _num_keys: u8,
    _security_flags: u8,
}

impl CmdAuthLogonProofClient {
    pub fn new(buf: &Vec<u8>) -> CmdAuthLogonProofClient {
        CmdAuthLogonProofClient {
            _opcode: buf[0],
            _client_public_key: [
                buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8], buf[9], buf[10],
                buf[11], buf[12], buf[13], buf[14], buf[15], buf[16], buf[17], buf[18], buf[19],
                buf[20], buf[21], buf[22], buf[23], buf[24], buf[25], buf[26], buf[27], buf[28],
                buf[29], buf[30], buf[31], buf[32],
            ],
            _client_proof: [
                buf[33], buf[34], buf[35], buf[36], buf[37], buf[38], buf[39], buf[40], buf[41],
                buf[42], buf[43], buf[44], buf[45], buf[46], buf[47], buf[48], buf[49], buf[50],
                buf[51], buf[52],
            ],
            _crc_hash: [
                buf[53], buf[54], buf[55], buf[56], buf[57], buf[58], buf[59], buf[60], buf[61],
                buf[62], buf[63], buf[64], buf[65], buf[66], buf[67], buf[68], buf[69], buf[70],
                buf[71], buf[72],
            ],
            _num_keys: buf[73],
            _security_flags: buf[74],
        }
    }
}

#[derive(Debug)]
pub struct CmdAuthLogonProofServer {
    _opcode: u8,
    _result: u8,
    _server_proof: [u8; 20],
    _account_flag: u32,
    _hardware_survey_id: u32,
    _unknown_flags: u16,
}

impl CmdAuthLogonProofServer {
    pub fn new(
        logon_proof_client: CmdAuthLogonProofClient,
        p: SrpProof,
    ) -> CmdAuthLogonProofServer {
        println!(
            "client public key: {:x?}",
            &logon_proof_client._client_public_key
        );
        println!("client proof: {:x?}", &logon_proof_client._client_proof);
        let (_, server_proof) = p
            .into_server(
                wow_srp::PublicKey::from_le_bytes(&logon_proof_client._client_public_key).unwrap(),
                logon_proof_client._client_proof,
            )
            .unwrap();

        CmdAuthLogonProofServer {
            _opcode: 1,
            _result: 0,
            _server_proof: server_proof,
            _account_flag: 0,
            _hardware_survey_id: 0,
            _unknown_flags: 0,
        }
    }

    pub async fn write(&self, socket: &mut TcpStream) -> Result<(), std::io::Error> {
        socket.write(&self._opcode.to_le_bytes()).await?;
        socket.write(&self._result.to_le_bytes()).await?;
        for i in self._server_proof.iter() {
            socket.write_all(&i.to_le_bytes()).await?;
        }
        socket.write(&self._account_flag.to_le_bytes()).await?;
        socket
            .write(&self._hardware_survey_id.to_le_bytes())
            .await?;
        socket.write(&self._unknown_flags.to_le_bytes()).await?;

        Ok(())
    }
}
