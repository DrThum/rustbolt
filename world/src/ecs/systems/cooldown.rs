use std::time::Instant;

use shipyard::{IntoIter, View, ViewMut};

use crate::{
    ecs::components::{cooldowns::Cooldowns, guid::Guid},
    entities::player::Player,
    protocol::{
        packets::{ClientSpellCooldown, SmsgSpellCooldown},
        server::ServerMessage,
    },
};

pub fn send_cooldowns(
    v_player: View<Player>,
    v_guid: View<Guid>,
    mut vm_cooldowns: ViewMut<Cooldowns>,
) {
    for (player, guid, cooldowns) in (&v_player, &v_guid, &mut vm_cooldowns).iter() {
        let now = Instant::now();

        let mut client_cooldowns: Vec<ClientSpellCooldown> = Vec::new();
        for (&spell_id, spell_cooldown) in cooldowns.list_mut().iter_mut() {
            if spell_cooldown.synced_with_client {
                continue;
            }

            client_cooldowns.push(ClientSpellCooldown {
                spell_id: *spell_id,
                cooldown_ms: spell_cooldown.end.duration_since(now).as_millis() as u32,
            });

            spell_cooldown.synced_with_client = true;
        }

        if !client_cooldowns.is_empty() {
            let packet = ServerMessage::new(SmsgSpellCooldown {
                guid: guid.0,
                flags: 0, // TODO: test with 1, 2 and 4
                cooldowns: client_cooldowns,
            });

            player.session.send(&packet).unwrap();
        }
    }
}
