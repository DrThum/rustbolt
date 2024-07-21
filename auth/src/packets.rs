use binrw::{binread, binrw, binwrite, NullString};
use wow_srp::{
    normalized_string::NormalizedString,
    server::{SrpProof, SrpServer, SrpVerifier},
};

#[binrw]
#[brw(repr(u8))]
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Opcode {
    CmdAuthLogonChallenge = 0x00,
    CmdAuthLogonProof = 0x01,
    // CMD_AUTH_RECONNECT_CHALLENGE = 0x02,
    CmdRealmList = 0x10,
}

#[binread]
#[derive(Debug)]
pub struct CmdAuthLogonChallengeClient {
    _opcode: Opcode,
    _protocol_version: u8,
    _size: u16,
    _game_name: NullString,
    _version: [u8; 3],
    _build: u16,
    _platform: NullString,
    _os: NullString,
    #[br(count = 4)]
    #[br(map = |s: Vec<u8>| String::from_utf8_lossy(&s).to_string())]
    _locale: String,
    _worldregion_bias: u32,
    _ip: [u8; 4], // u32 on wowdev.wiki
    _account_name_length: u8,
    pub account_name: NullString,
}

#[binwrite]
#[derive(Debug)]
pub struct CmdAuthLogonChallengeServer {
    _opcode: Opcode,
    _protocol_version: u8,
    _result: u8,
    _server_public_key: [u8; wow_srp::PUBLIC_KEY_LENGTH as usize],
    _generator_len: u8, // Always 1
    _generator: u8,
    _large_safe_prime_len: u8,
    _large_safe_prime: [u8; wow_srp::LARGE_SAFE_PRIME_LENGTH as usize],
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

    pub fn new(username: &NullString) -> (CmdAuthLogonChallengeServer, SrpProof) {
        let p = Self::get_proof(&username.to_string());

        (
            CmdAuthLogonChallengeServer {
                _opcode: Opcode::CmdAuthLogonChallenge,
                _protocol_version: 0,
                _result: 0,
                _server_public_key: *p.server_public_key(),
                _generator_len: 1,
                _generator: wow_srp::GENERATOR,
                _large_safe_prime_len: wow_srp::LARGE_SAFE_PRIME_LENGTH,
                _large_safe_prime: wow_srp::LARGE_SAFE_PRIME_LITTLE_ENDIAN,
                _salt: *p.salt(),
                _crc_salt: [0; 16],
                _security_flags: 0,
            },
            p,
        )
    }
}

#[binread]
#[derive(Debug)]
pub struct CmdAuthLogonProofClient {
    _opcode: Opcode,
    _client_public_key: [u8; 32],
    _client_proof: [u8; 20],
    _crc_hash: [u8; 20],
    _num_keys: u8,
    _security_flags: u8,
}

#[binwrite]
#[derive(Debug)]
pub struct CmdAuthLogonProofServer {
    _opcode: Opcode,
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
    ) -> (CmdAuthLogonProofServer, SrpServer) {
        let (srp_server, server_proof) = p
            .into_server(
                wow_srp::PublicKey::from_le_bytes(&logon_proof_client._client_public_key).unwrap(),
                logon_proof_client._client_proof,
            )
            .unwrap();

        (
            CmdAuthLogonProofServer {
                _opcode: Opcode::CmdAuthLogonProof,
                _result: 0,
                _server_proof: server_proof,
                _account_flag: 0,
                _hardware_survey_id: 0,
                _unknown_flags: 0,
            },
            srp_server,
        )
    }
}

#[binread]
#[derive(Debug)]
pub struct CmdRealmListClient {
    _opcode: Opcode,
    _padding: u32,
}

#[binrw]
#[brw(repr(u8))]
#[derive(Debug)]
pub enum RealmType {
    Normal = 0,
    PvP = 1,
    RolePlay = 6,
    RolePlayPvP = 8,
}

impl TryFrom<i32> for RealmType {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(RealmType::Normal),
            1 => Ok(RealmType::PvP),
            6 => Ok(RealmType::RolePlay),
            8 => Ok(RealmType::RolePlayPvP),
            _ => Err(format!("Invalid realm type: {}", value)),
        }
    }
}

#[binrw]
#[brw(repr(u8))]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum RealmFlag {
    None = 0x00,
    Invalid = 0x01,
    Offline = 0x02,
    SpecifyBuild = 0x04,
    ForceRecommended = 0x20,
    ForceNewPlayers = 0x40,
    ForceFull = 0x80,
}

#[binwrite]
#[derive(Debug)]
pub struct Realm {
    pub realm_type: RealmType,
    #[bw(map = |l: &bool| if *l { 1_u8 } else { 0_u8 })]
    pub locked: bool,
    pub realm_flags: u8,
    pub realm_name: NullString,
    pub address_port: NullString,
    pub population: f32,
    pub num_chars: u8,
    pub realm_category: u8, // https://github.com/mangosone/server/blob/d62fdfe93b96bef5daa36433116d2f0eeb9fc3d0/src/game/WorldHandlers/World.h#L411-L452
    pub realm_id: u8,
}

impl Realm {
    pub fn size(&self) -> u16 {
        (10 + self.realm_name.len() + 1 + self.address_port.len() + 1)
            .try_into()
            .unwrap()
    }
}

#[binwrite]
#[derive(Debug)]
pub struct CmdRealmListServer<'a> {
    pub opcode: Opcode,
    pub size: u16,
    pub padding: u32,
    pub num_realms: u16,
    pub realms: &'a Vec<Realm>,
    pub padding_footer: u16,
}
