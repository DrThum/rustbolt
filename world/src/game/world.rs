use std::time::{Duration, SystemTime};

use tokio::time::interval;

use crate::{config::WorldConfig, datastore::DataStore};

pub struct World {
    pub data_store: DataStore,
}

impl World {
    pub fn new(config: &WorldConfig) -> Self {
        let data_store = DataStore::load_dbcs(&config.common.data).expect("Unable to load DBCs");

        World { data_store }
    }

    pub async fn start(&'static self) {
        tokio::spawn(async move {
            self.game_loop().await;
        });
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

    fn tick(&self, _diff: Duration) {}
}
