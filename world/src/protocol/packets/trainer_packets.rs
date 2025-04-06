use binrw::{binwrite, NullString};
use opcode_derive::server_opcode;

use crate::entities::object_guid::ObjectGuid;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::TrainerSpellState;

#[binwrite]
#[server_opcode]
pub struct SmsgTrainerBuySucceeded {
    pub trainer_guid: ObjectGuid,
    pub spell_id: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgTrainerList {
    pub trainer_guid: ObjectGuid,
    pub trainer_type: u32,
    pub spell_count: u32,
    pub spells: Vec<TrainerSpell>,
    pub title: NullString,
}

#[binwrite]
#[derive(Debug)]
pub struct TrainerSpell {
    pub spell_id: u32,
    #[bw(map = |b: &TrainerSpellState| *b as u8)]
    pub state: TrainerSpellState,
    pub cost: u32,
    #[bw(map = |b: &bool| if *b { 1_u32 } else { 0_u32 })]
    pub can_learn_primary_profession_first_rank: bool,
    #[bw(map = |b: &bool| if *b { 1_u32 } else { 0_u32 })]
    pub enable_learn_primary_profession_button: bool,
    pub required_level: u8,
    pub required_skill: u32,
    pub required_skill_value: u32,
    pub previous_spell: u32, // chain_node ? (chain_node->prev ? chain_node->prev : chain_node->req) : 0
    pub required_required_spell: u32, // chain_node && chain_node->prev ? chain_node->req : 0
    pub unk: u32,            // always 0 in MaNGOS
}
