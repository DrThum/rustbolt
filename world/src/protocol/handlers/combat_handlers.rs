use shipyard::{View, ViewMut};

use crate::ecs::components::guid::Guid;
use crate::ecs::components::melee::Melee;
use crate::ecs::components::unit::Unit;
use crate::entities::object_guid::ObjectGuid;
use crate::entities::player::Player;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_attack_swing(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgAttackSwing = ClientMessage::read_as(data).unwrap();
        if let Some(map) = session.current_map() {
            if let Some(player_ecs_entity) = session.player_entity_id() {
                map.world().run(
                    |mut vm_melee: ViewMut<Melee>, vm_player: ViewMut<Player>| {
                        vm_melee[player_ecs_entity].is_attacking = true;

                        let player = &vm_player[player_ecs_entity];
                        if !player.is_in_combat_with(&cmsg.guid) {
                            player.set_in_combat_with(cmsg.guid);
                        }
                    },
                );
            }

            let player_guid = session.player_guid().unwrap();
            let packet = ServerMessage::new(SmsgAttackStart {
                attacker_guid: player_guid,
                target_guid: cmsg.guid,
            });

            map.broadcast_packet(&player_guid, &packet, None, true);
        }
    }

    pub(crate) fn handle_cmsg_attack_stop(PacketHandlerArgs { session, .. }: PacketHandlerArgs) {
        if let Some(map) = session.current_map() {
            let player_guid = session.player_guid().unwrap();
            if let Some(player_ecs_entity) = map.lookup_entity_ecs(&player_guid) {
                let target_guid = map.world().run(
                    |mut vm_melee: ViewMut<Melee>, v_unit: View<Unit>, v_guid: View<Guid>| {
                        vm_melee[player_ecs_entity].is_attacking = false;
                        v_unit[player_ecs_entity]
                            .target()
                            .map(|target_entity_id| v_guid[target_entity_id].0)
                    },
                );

                let packet = ServerMessage::new(SmsgAttackStop {
                    attacker_guid: player_guid.as_packed(),
                    enemy_guid: target_guid.unwrap_or(ObjectGuid::zero()).as_packed(),
                    unk: 0,
                });

                map.broadcast_packet(&player_guid, &packet, None, true);
            }
        }
    }
}
