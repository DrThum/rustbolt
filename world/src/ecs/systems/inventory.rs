use shipyard::{IntoIter, ViewMut};

use crate::{entities::player::Player, protocol::packets::SmsgUpdateObject};

pub fn send_inventory_update(mut vm_player: ViewMut<Player>) {
    for player in (&mut vm_player).iter() {
        let updates = player.get_inventory_updates_and_reset();

        let smsg_update_object = SmsgUpdateObject {
            updates_count: updates.len() as u32,
            has_transport: false,
            updates,
        };

        player.session.update_entity(smsg_update_object);
    }
}
