use shipyard::{AllStoragesViewMut, UniqueView};

use crate::game::{map::WrappedMap, world_context::WrappedWorldContext};

pub fn process_packets(vm_all_storages: AllStoragesViewMut) {
    vm_all_storages.run(
        |map: UniqueView<WrappedMap>, world_context: UniqueView<WrappedWorldContext>| {
            let world_context = world_context.0.clone();

            for (session, packet) in map.0.get_and_reset_queued_packets() {
                let (_, handler) = world_context
                    .opcode_handler
                    .get_handler(packet.header.opcode);

                handler(session.clone(), world_context.clone(), packet.payload);
            }
        },
    );
}
