use binrw::io::Cursor;
use binrw::{binread, binwrite, BinWrite, BinWriterExt, NullString};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::opcodes::Opcode;

pub struct ServerMessage<const OPCODE: u16, Payload: WorldPacketPayload<OPCODE> + BinWrite> {
    payload: Payload,
}

pub trait WorldPacketPayload<const OPCODE: u16> {
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

impl<const OPCODE: u16, Payload: WorldPacketPayload<OPCODE> + BinWrite>
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
}

// pub struct ClientMessage<P: WorldPacketPayload + BinRead> { opcode, payload }

#[binwrite]
struct ServerMessageHeader {
    #[bw(big)]
    pub size: u16,
    pub opcode: u16,
}

#[binwrite]
pub struct SmsgAuthChallenge {
    pub server_seed: u32,
}

impl WorldPacketPayload<{ Opcode::SmsgAuthChallenge as u16 }> for SmsgAuthChallenge {}

#[binread]
#[derive(Debug)]
pub struct CmsgAuthSession {
    #[br(big)]
    pub _size: u16,
    pub _opcode: u32,
    pub _build: u32,
    pub _server_id: u32,
    pub _username: NullString,
    pub _client_seed: u32,
    pub _client_proof: [u8; 20],
    pub _decompressed_addon_info_size: u32,
    #[br(count = _size - (4 + 4 + 4 + (_username.len() - 1) + 4 + 20 + 4) as u16)]
    pub _compressed_addon_info: Vec<u8>,
}

#[binwrite]
pub struct SmsgAuthResponse {
    pub _header: Vec<u8>, // Encrypted size + opcode
    pub _result: u8,
    pub _billing_time: u32,
    pub _billing_flags: u8,
    pub _billing_rested: u32,
}

#[binwrite]
pub struct SmsgCharEnum {
    pub header: Vec<u8>,
    pub amount_of_characters: u8,
    pub character_guid: u64, // TODO: move these to a separate struct as there can be many
    pub character_name: NullString,
    pub character_race: u8,
    pub character_class: u8,
    pub character_gender: u8,
    pub character_skin: u8,
    pub character_face: u8,
    pub character_hairstyle: u8,
    pub character_haircolor: u8,
    pub character_facialstyle: u8,
    pub character_level: u8,
    pub character_area: u32,
    pub character_map: u32,
    pub character_position_x: f32,
    pub character_position_y: f32,
    pub character_position_z: f32,
    pub character_guild_id: u32,
    pub character_flags: u32,
    pub character_first_login: u8, // FIXME: bool
    pub character_pet_display_id: u32,
    pub character_pet_level: u32,
    pub character_pet_family: u32,
    pub character_equip_head: u32,
    pub character_equip_head_slot: u8, // 1
    pub character_equip_head_enchant: u32,
    pub character_equip_neck: u32,
    pub character_equip_neck_slot: u8, // 2
    pub character_equip_neck_enchant: u32,
    pub character_equip_shoulders: u32,
    pub character_equip_shoulders_slot: u8, // 3
    pub character_equip_shoulders_enchant: u32,
    pub character_equip_body: u32,
    pub character_equip_body_slot: u8, // 4
    pub character_equip_body_enchant: u32,
    pub character_equip_chest: u32,
    pub character_equip_chest_slot: u8, // 5
    pub character_equip_chest_enchant: u32,
    pub character_equip_waist: u32,
    pub character_equip_waist_slot: u8, // 6
    pub character_equip_waist_enchant: u32,
    pub character_equip_legs: u32,
    pub character_equip_legs_slot: u8, // 7
    pub character_equip_legs_enchant: u32,
    pub character_equip_feet: u32,
    pub character_equip_feet_slot: u8, // 8
    pub character_equip_feet_enchant: u32,
    pub character_equip_wrists: u32,
    pub character_equip_wrists_slot: u8, // 9
    pub character_equip_wrists_enchant: u32,
    pub character_equip_hands: u32,
    pub character_equip_hands_slot: u8, // 10
    pub character_equip_hands_enchant: u32,
    pub character_equip_finger1: u32,
    pub character_equip_finger1_slot: u8, // 11
    pub character_equip_finger1_enchant: u32,
    pub character_equip_finger2: u32,
    pub character_equip_finger2_slot: u8, // 11
    pub character_equip_finger2_enchant: u32,
    pub character_equip_trinket1: u32,
    pub character_equip_trinket1_slot: u8, // 12
    pub character_equip_trinket1_enchant: u32,
    pub character_equip_trinket2: u32,
    pub character_equip_trinket2_slot: u8, // 12
    pub character_equip_trinket2_enchant: u32,
    pub character_equip_back: u32,
    pub character_equip_back_slot: u8, // 16
    pub character_equip_back_enchant: u32,
    pub character_equip_mainhand: u32,
    pub character_equip_mainhand_slot: u8, // 21
    pub character_equip_mainhand_enchant: u32,
    pub character_equip_offhand: u32,
    pub character_equip_offhand_slot: u8, // 22
    pub character_equip_offhand_enchant: u32,
    pub character_equip_ranged: u32,
    pub character_equip_ranged_slot: u8, // 26
    pub character_equip_ranged_enchant: u32,
    pub character_equip_tabard: u32,
    pub character_equip_tabard_slot: u8, // 19
    pub character_equip_tabard_enchant: u32,
    pub character_first_bag_display_id: u32,    // Always 0
    pub character_first_bag_inventory_type: u8, // Always 0
    pub unk_0: u32,                             // Always 0
}
