use shipyard::UniqueView;

use crate::{
    game::{packet_queue::WrappedPacketQueue, world_context::WrappedWorldContext},
    session::opcode_handler::PacketHandlerArgs,
};

pub fn process_packets(
    packet_queue: UniqueView<WrappedPacketQueue>,
    world_context: UniqueView<WrappedWorldContext>,
) {
    let world_context = world_context.0.clone();

    for (session, packet) in packet_queue.get_packets_and_reset_queue() {
        let (_, handler) = world_context
            .opcode_handler
            .get_handler(packet.header.opcode);

        handler(PacketHandlerArgs {
            session: session.clone(),
            world_context: world_context.clone(),
            data: packet.payload,
        });
    }
}
