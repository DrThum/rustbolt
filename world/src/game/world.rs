use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::time::interval;

use crate::{config::WorldConfig, datastore::DataStore};

pub struct World {
    pub data_store: DataStore,
    start_time: Instant,
    config: Arc<WorldConfig>,
}

impl World {
    pub fn new(config: Arc<WorldConfig>, pool: Arc<Pool<SqliteConnectionManager>>) -> Self {
        let conn = pool.get().unwrap();
        let data_store = DataStore::load_data(&config.common.data, &conn)
            .expect("Error when loading static data");

        World {
            data_store,
            start_time: Instant::now(),
            config,
        }
    }

    pub async fn start(&'static self) {
        tokio::spawn(async move {
            self.game_loop().await;
        });
    }

    // Return the elapsed time since the World started
    pub fn game_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn config(&self) -> &WorldConfig {
        &self.config
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
