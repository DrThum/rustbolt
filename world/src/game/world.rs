use std::{sync::Arc, time::Duration};

use tokio::{
    sync::RwLock,
    time::{interval, Instant},
};

use super::world_context::WorldContext;

pub struct World {
    world_context: Arc<WorldContext>,
}

impl World {
    pub fn new(world_context: Arc<WorldContext>) -> Self {
        World { world_context }
    }

    pub async fn start(world: Arc<RwLock<World>>) {
        tokio::spawn(async move {
            world.read().await.game_loop().await;
        });
    }

    async fn game_loop(&self) {
        let mut interval = interval(Duration::from_millis(50));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut time = Instant::now();

        loop {
            let new_time = Instant::now();
            let diff = new_time.duration_since(time);

            time = new_time;
            self.tick(diff).await;

            interval.tick().await;
        }
    }

    async fn tick(&self, diff: Duration) {
        self.world_context
            .map_manager
            .tick(diff, self.world_context.clone())
            .await;
    }
}
