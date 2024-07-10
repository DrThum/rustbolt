use std::sync::atomic::AtomicUsize;

use shipyard::Component;

#[derive(Component, Default)]
pub struct NearbyPlayers {
    pub count: AtomicUsize,
}
