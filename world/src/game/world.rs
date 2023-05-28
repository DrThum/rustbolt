use std::{sync::Arc, time::Duration};

use tokio::{
    sync::RwLock,
    time::{interval, Instant},
};

use crate::{config::WorldConfig, SessionHolder};

use super::world_context::WorldContext;

pub struct World {
    world_context: Arc<WorldContext>,
    session_holder: Arc<SessionHolder>,
}

impl World {
    pub fn new(
        _start_time: Instant,
        _config: Arc<WorldConfig>,
        world_context: Arc<WorldContext>,
        session_holder: Arc<SessionHolder>,
    ) -> Self {
        World {
            session_holder,
            world_context,
        }
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
        self.session_holder.tick(self.world_context.clone()).await;
        self.world_context.map_manager.tick(diff).await;
    }
}
