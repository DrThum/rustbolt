use std::sync::Arc;

use binrw::NullString;

use crate::{
    game::world_context::WorldContext,
    protocol::{
        client::ClientMessage,
        packets::{
            CmsgCreatureQuery, SmsgCreatureQueryResponse, SmsgCreatureQueryResponseUnknownTemplate,
        },
        server::ServerMessage,
    },
    session::{opcode_handler::OpcodeHandler, world_session::WorldSession},
};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_creature_query(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgCreatureQuery = ClientMessage::read_as(data).unwrap();

        if let Some(template) = world_context.data_store.get_creature_template(cmsg.entry) {
            let packet = ServerMessage::new(SmsgCreatureQueryResponse {
                entry: template.entry,
                name: NullString::from(template.name.clone()),
                name2: 0,
                name3: 0,
                name4: 0,
                sub_name: NullString::from(template.sub_name.clone().unwrap_or("".to_owned())),
                icon_name: NullString::from(template.icon_name.clone().unwrap_or("".to_owned())),
                type_flags: template.type_flags,
                type_id: template.type_id,
                family: template.family,
                rank: template.rank,
                unk: 0,
                pet_spell_data_id: template.pet_spell_data_id,
                model_ids: template.model_ids.clone(),
                health_multiplier: template.health_multiplier,
                power_multiplier: template.power_multiplier,
                racial_leader: template.racial_leader,
            });

            session.send(&packet).unwrap();
        } else {
            let packet = ServerMessage::new(SmsgCreatureQueryResponseUnknownTemplate {
                masked_entry: cmsg.entry | 0x80000000,
            });

            session.send(&packet).unwrap();
        }
    }
}
