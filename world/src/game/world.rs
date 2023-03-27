use std::time::{Duration, SystemTime};

use tokio::time::interval;

pub struct World {}

impl World {
    pub fn new() -> Self {
        World {}
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            self.game_loop().await;
        })
        .await
        .expect("World loop error");
    }

    async fn game_loop(&self) {
        let mut interval = interval(Duration::from_millis(50));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut time = SystemTime::now();

        loop {
            let new_time = SystemTime::now();
            if let Ok(diff) = new_time.duration_since(time) {
                time = new_time;
                self.tick(diff);
            }

            interval.tick().await;
        }
    }

    fn tick(&self, diff: Duration) {}
}
