use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::{sync::RwLock, time::interval};

use crate::config::WorldConfig;

pub struct World {
    start_time: Instant,
    config: Arc<WorldConfig>,
}

impl World {
    pub fn new(start_time: Instant, config: Arc<WorldConfig>) -> Self {
        World { start_time, config }
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
            self.tick(diff);

            interval.tick().await;
        }
    }

    fn tick(&self, _diff: Duration) {}
}
