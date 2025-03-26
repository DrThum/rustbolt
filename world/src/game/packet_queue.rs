use std::{collections::VecDeque, sync::Arc};

use parking_lot::RwLock;

use crate::{
    create_wrapped_resource, protocol::client::ClientMessage, session::world_session::WorldSession,
};

pub struct PacketQueue {
    queue: RwLock<VecDeque<(Arc<WorldSession>, ClientMessage)>>,
}

impl PacketQueue {
    pub fn new() -> Self {
        Self {
            queue: RwLock::new(VecDeque::new()),
        }
    }

    pub fn queue_packet(&self, world_session: Arc<WorldSession>, packet: ClientMessage) {
        self.queue.write().push_back((world_session, packet));
    }

    pub fn get_packets_and_reset_queue(&self) -> VecDeque<(Arc<WorldSession>, ClientMessage)> {
        let mut guard = self.queue.write();
        let packets = (*guard).clone();
        *guard = VecDeque::new();

        packets
    }
}

create_wrapped_resource!(WrappedPacketQueue, PacketQueue);
